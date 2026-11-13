# ducky-core

Domain vocabulary shared by every ducky crate. No dependencies, no IO.

## Public API

- Profiles: `ProfileKind`, `Profile`
- Session: `SessionInfo`
- Catalog: `RelationKind`, `RelationRef`, `Column`, `Relation`, `Schema`, `Catalog` (`schema`, `relation` lookups), `Index`, `ForeignKey`, `RelationDetails`
- Data: `Cell`, `ResultSet`, `SortDirection`, `Sort`, `PageRequest`, `Page`, `DEFAULT_PAGE_SIZE` (50), `MAX_PAGE_SIZE` (5000)
- Scripts: `TransactionMode` (default `Transaction`), `StatementOutcome`, `StatementResult`, `ScriptFailure`, `ScriptReport`
- Files: `FileEntry`, `ToolStatus`, `PgTools`, `RestoreOptions` (default all false), `StoragePaths`

## Tests

```sh
cargo test -p ducky-core
```

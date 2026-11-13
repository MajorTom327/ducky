# ducky-sql

View-agnostic SQL text services for ducky: positions, syntax highlighting, statement splitting, identifier quoting and completion. Built on the `sqlparser` tokenizer with the PostgreSQL dialect; it never parses full statements, so it works on incomplete or invalid text while the user types.

## Public API

- `TextPosition { line, column }`: 0-based line, column counted in chars.
- `offset_at(sql, position) -> usize`: byte offset of a position, clamped to the line end and to `sql.len()`.
- `position_at(sql, offset) -> TextPosition`: position of a byte offset (clamped, floored to a char boundary).
- `TokenKind`: `Keyword`, `Identifier`, `QuotedIdentifier`, `String`, `Number`, `Comment`, `Operator`, `Punctuation`.
- `HighlightSpan { start, end, kind }`: `end` is exclusive; a span may cover several lines.
- `highlight(sql) -> Vec<HighlightSpan>`: one span per non-whitespace token. When the tokenizer fails (unterminated string, quoted identifier, dollar quote or block comment), the untokenized tail becomes one `String` or `Comment` span.
- `split_statements(sql) -> Vec<String>`: cuts at top-level semicolons, ignoring semicolons inside strings, comments, dollar quotes and `BEGIN ATOMIC … END` bodies. Pieces are trimmed, without their semicolon; whitespace- or comment-only pieces are dropped. Untokenizable trailing text is kept so PostgreSQL reports the syntax error.
- `quote_identifier(identifier)`: always double-quotes, doubling inner quotes.
- `quote_identifier_if_needed(identifier)`: bare when it matches `^[a-z_][a-z0-9_]*$`, quoted otherwise.
- `quote_literal(value)`: standard SQL string literal, doubling inner single quotes (`it's` → `'it''s'`).
- `CompletionKind`, `CompletionItem { label, kind, detail }`, `Completion { replace_start, items }`.
- `complete(sql, cursor, catalog, default_schema) -> Completion`: candidates for the identifier ending at the byte offset `cursor`; the caller replaces `sql[replace_start..cursor]` with the chosen label.
- `find_parameters(sql) -> Vec<String>`: query parameters of a script, unique, in first-appearance order. Named parameters are `:name` (a `:` directly followed by `[A-Za-z_][A-Za-z0-9_]*`) and keep their colon (`":id"`); positional ones are `$1`, `$2`, …. Casts (`::int`), strings, quoted identifiers, comments and dollar-quoted bodies are ignored. Inside `[ ]`, a `:` following a value (number, word, parameter, `)` or `]`) is an array-slice separator (`arr[1:n]`, `arr[lo:hi]`); after `[` or `,` it starts a parameter (`ARRAY[:a, :b]`).
- `ParameterValue`: `Text(value)` binds `quote_literal(value)` (PostgreSQL coerces the unknown literal); `Raw(sql)` is inserted verbatim (`now()`, `NULL`, `(1, 2, 3)`).
- `bind_parameters(sql, values) -> String`: replaces every occurrence of each parameter found by `find_parameters` with its value from `values` (`(name, value)` pairs keyed like `":id"` or `"$1"`), using token byte offsets; parameters without a value are left untouched.

## Completion rules

- After `qualifier.`: relations of the schema named `qualifier`, else columns of the relation aliased or named `qualifier` in the current statement.
- After `FROM`, `JOIN`, `INTO`, `UPDATE`, `TABLE`, `TRUNCATE`: relations of the default schema, then schema names.
- Elsewhere: columns of the relations referenced by the current statement, then relations of the default schema, then SQL keywords; with an empty prefix only the columns.
- Items match the prefix case-insensitively, are sorted inside each group and capped at 50.

## Tests

```sh
cargo test -p ducky-sql
```

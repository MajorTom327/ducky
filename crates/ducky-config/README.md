# ducky-config

Loads ducky's TOML configuration: connection profiles and storage directories.

## Lookup order

1. `--config PATH` (explicit, must exist)
2. `./ducky.toml`
3. `$XDG_CONFIG_HOME/ducky/config.toml`, or `$HOME/.config/ducky/config.toml` when `XDG_CONFIG_HOME` is unset

## Format

```toml
sql_dir = "sql"      # optional, relative to the config file
dump_dir = "dumps"   # optional, relative to the config file
page_size = 50       # optional, rows per table page, 1–5000

[[profiles]]
label = "Prod"
project = "soiltrackers"   # optional, trimmed, blank = none
type = "prod"              # local | staging | prod
tags = ["eu", "readonly"]  # optional, trimmed, lowercased, deduplicated
url = "postgres://app:${PROD_DB_PASSWORD}@db/app?sslmode=require"
```

Labels are unique across the file. An empty tag is rejected with `ConfigError::EmptyTag { label }`.

`${VAR}` is validated at load time and expanded at connect time with `expand_env`.

## Public API

- `Config { source, profiles, storage, page_size }`
- `ConfigError` (including `InvalidPageSize { page_size }` and `EmptyTag { label }`)
- `SearchContext { explicit_path, working_dir, xdg_config_home, home_dir }`, `SearchContext::from_process`
- `resolve_config_path(&SearchContext)`
- `load(&Path)`
- `expand_env(raw_url, lookup)`

## Tests

```sh
cargo test -p ducky-config
```

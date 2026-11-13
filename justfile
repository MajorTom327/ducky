database_url := env("DUCKY_TEST_DATABASE_URL", "postgres://ducky:ducky@localhost:5440/ducky")

default:
    @just --list

build:
    cargo build --workspace

release:
    cargo build --workspace --release

fmt:
    cargo fmt --all

lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

db-up:
    docker compose up -d --wait postgres

db-down:
    docker compose down

test-db: db-up
    DUCKY_TEST_DATABASE_URL={{database_url}} cargo test --workspace -- --ignored

dev *args:
    cargo run -p ducky-tui -- --config dev/ducky.dev.toml {{args}}

ci: lint test release

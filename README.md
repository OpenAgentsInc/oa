# OpenAgents.com

Our main website & web app.

## Important commands

- `cargo watch -x check`
- `cargo watch -x check -x test -x run`
- `docker build --tag oa .`
- `DATABASE_URL=postgres://... cargo sqlx migrate run`
- `DATABASE_URL=postgresql://postgres:password@127.0.0.1:5432/newsletter cargo sqlx prepare --workspace -- --all-targets`
- `cargo run --bin generate-hierarchy`

## Development Setup

This project uses git hooks for code quality. See [docs/githooks.md](docs/githooks.md) for setup instructions.

# VaaS Server

TODO

## Setup

```bash
docker-compose up -d # Start postgres
cargo install --version 0.1.0-beta.1 sqlx-cli # adds cargo sqlx command
cargo sqlx migrate run # apply all migrations
cargo run
```

# Run tests

```bash
docker-compose up -d # Start postgres
cargo test
```

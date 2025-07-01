# `ftdemo`

This crate combines the SQLite implementation of `fts-sqlite` with the REST API  of `fts-axum` to create a binary suitable for interactive demonstration of flow trading functionality. As suggested by the name, correctness and simplicity are prioritized over performance, though the use of SQLite nevertheless enables very fast operations.

It is recommended to pair this binary with a frontend client, such as [this one](https://github.com/forward-market-design/flow-trading-client). The client provides a graphical, administrative interface for familiarizing oneself with the primitives and operations of flow trading and how a forward market might be built upon this foundation.

## Installation and Usage

The binary can be installed as simply as `cargo install ftdemo`. Once installed, running the server looks like this:
```bash
APP_SECRET=SECRET ftdemo serve --config ./path/to/config.toml
```

The two key things are setting the HMAC secret for JWT authentication, and the configuration file `./path/to/config.toml`. This file looks like:

```toml
# HTTP Server Configuration
[server]
# The address and port to bind the server to
bind_address = "0.0.0.0:8080"

# Database Configuration
[database]
# Path to the SQLite database file (If not specified, uses an in-memory database)
#database_path = "./dev.db"
  
# Whether to create the database if it doesn't exist
create_if_missing = true

[schedule]
# What "anchor" time to start the auctions from?
from = "2025-01-01T00:00:00Z"

# How often to run a batch auction?
every = "10s"
```

All the configuration options may alternatively be specified by environment variables `APP_[SERVER|DATABASE|SCHEDULE]__[VARNAME]`.

Once running, the server will respond to requests sent to the bind address. If the bind address is 0.0.0.0:8080, then browsing to http://localhost:8080/docs will show an interactive API explorer if the server is successfully running. 
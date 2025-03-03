# Marketplace

## Repositories

This crate defines a number of repository traits, which collectively define
all the IO operations an implementation needs to support. This allows an
implementation to easily switch out the database or combine multiple storage
technologies as needed.

## Server

This crate leverages [Axum](https://crates.io/crates/axum) to implement an API
server that is generic over the `MarketRepository` implementation. For
documentation on the available routes, launch a server and view
http://localhost:8080/rapidoc (or whatever port you configure the server to run
on), which consumes the OpenAPI specification available at
http://localhost:8080/api-docs/openapi.json.

# FTS Server

This crate provides a REST API for the core flow trading operations. Building
the sibling crate `fts-demo` will generate an OpenAPI schema. A running server will host this schema at http://localhost:8080/rapidoc.

## On the use of JSON and HTTP

It is true that JSON is a significantly flawed choice for (de)serialization of
bid data. It is also true that a RESTful API over HTTP is questionable, at
best, with respect to building a trading platform. On the other hand, these
choices allow for virtually any programming environment to easily interface
with the server, as well as open the door to rich, web-based clients.

Given that this project is primarily intended to *motivate* the use of flow trading, especially in the context of forward markets, these trade-offs are more than reasonable. With that said, the design of flow trading specifically discourages high-frequency execution, so the performance overhead of these trade-offs are also largely irrelevant.

## Authorization

In the interest of simplicity, endpoints that process bid data (or execute administrative actions) expect HTTP requests to contain the `Authorization` header with a JWT bearer token. The `sub:` claim of this token must be the bidder's UUID. To authorize an administrative action, this token must contain the custom claim `admin: true`. It is left to the operator to securely authenticate and generate these tokens.

## API Endpoints and Data Types

Please refer to the automatically generated OpenAPI schema for up-to-date documentation of the endpoints. Note that any endpoint expecting a datetime type expects an RFC3339-compliant string.
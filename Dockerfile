# Dockerfile adapted from:
# https://github.com/LukeMathWalker/cargo-chef/blob/v0.1.64/README.md#without-the-pre-built-image

# cargo-chef is a nice tool that lets us cache build dependencies and produce
# lean images.

FROM rust:1-slim-bookworm AS chef 
RUN cargo install cargo-chef 
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin fts-demo

FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/fts-demo /usr/local/bin
ENTRYPOINT ["/usr/local/bin/fts-demo"]

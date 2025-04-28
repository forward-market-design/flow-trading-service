# Dockerfile adapted from:
# https://github.com/LukeMathWalker/cargo-chef/blob/v0.1.64/README.md#running-the-binary-in-alpine

# cargo-chef is a nice tool that lets us cache build dependencies and produce
# lean images.

# Build argument for target architecture
# If you're on another architecture, pass this as a build arg
ARG RUST_TARGET=x86_64-unknown-linux-musl

FROM clux/muslrust:stable AS chef
USER root
RUN cargo install cargo-chef 
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG RUST_TARGET
COPY --from=planner /app/recipe.json recipe.json
# Use the target architecture from build arg
RUN cargo chef cook --release --target ${RUST_TARGET} --recipe-path recipe.json
COPY . .
RUN cargo build --release --target ${RUST_TARGET} --bin ftdemo

FROM alpine AS runtime
ARG RUST_TARGET
RUN addgroup -S ftsuser && adduser -S ftsuser -G ftsuser

# Open source project metadata
LABEL org.opencontainers.image.source="https://github.com/forward-market-design/flow-trading-service"
LABEL org.opencontainers.image.description="Flow Trading Service Demo"
LABEL org.opencontainers.image.licenses="MIT"

# Remove the unnecessary step since we're using the full target path
COPY --from=builder /app/target/${RUST_TARGET}/release/ftdemo /usr/local/bin/
USER ftsuser

# Container listens on port 8080
EXPOSE 8080

# TODO: Add healthcheck?

ENTRYPOINT ["/usr/local/bin/ftdemo"]

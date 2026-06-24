# Stage 1: build the contract WASM
FROM rust:1.82.0-slim AS builder

WORKDIR /app

# Install the WASM target required by Soroban
RUN rustup target add wasm32v1-none

# Copy workspace manifests and lock file first for layer caching
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY contracts/ contracts/

# Build the release WASM artifact
RUN cargo build --target wasm32v1-none --release -p price-oracle

# Stage 2: minimal image containing only the WASM artifact
FROM scratch AS final
COPY --from=builder /app/target/wasm32v1-none/release/price_oracle.wasm /price_oracle.wasm

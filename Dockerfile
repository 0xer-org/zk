FROM rust:1.84-slim-bookworm AS builder
WORKDIR /app

# Install build dependencies first (cached layer)
RUN apt-get update && apt-get install -y pkg-config libssl-dev protobuf-compiler build-essential

# Copy dependency files only
COPY Cargo.toml Cargo.lock rust-toolchain ./
COPY lib/Cargo.toml lib/
COPY app/Cargo.toml app/
COPY prover/Cargo.toml prover/

# Create dummy source files to build dependencies
RUN mkdir -p lib/src app/src prover/src && \
    touch lib/src/lib.rs app/src/lib.rs && \
    echo "fn main() {}" > prover/src/main.rs

# Build dependencies (this layer is cached if Cargo.toml unchanged)
RUN cargo build --release --bin prover 2>/dev/null || true

# Copy actual source code
COPY lib/src lib/src
COPY app/src app/src
COPY app/elf app/elf
COPY prover/src prover/src
COPY prover/data prover/data

# Build the binary (only recompiles changed code)
RUN cargo build --release --bin prover

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libssl-dev ca-certificates curl gnupg && \
    curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/debian bookworm stable" > /etc/apt/sources.list.d/docker.list && \
    apt-get update && apt-get install -y docker-ce-cli && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/prover /app/prover
COPY --from=builder /app/app/elf /app/app/elf
COPY --from=builder /app/prover/data/vm_pk /app/data/vm_pk
COPY --from=builder /app/prover/data/vm_vk /app/data/vm_vk

CMD ["./prover"]

FROM rust:1.84-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y pkg-config libssl-dev protobuf-compiler build-essential
RUN cargo build --release --bin prover

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libssl-dev ca-certificates docker.io && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/prover /app/prover
COPY --from=builder /app/app/elf /app/app/elf
COPY --from=builder /app/prover/data/vm_pk /app/data/vm_pk
COPY --from=builder /app/prover/data/vm_vk /app/data/vm_vk

CMD ["./prover"]
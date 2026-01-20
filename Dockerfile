FROM rust:1.84-slim-bookworm as builder
WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y pkg-config libssl-dev protobuf-compiler
RUN cargo build --release --bin prover

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/prover /app/prover
COPY --from=builder /app/app/elf /app/app/elf
RUN mkdir -p /app/prover/data

CMD ["./prover"]
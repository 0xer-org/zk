# Prover Tools

## Binary Structure

This workspace contains multiple binaries:

- **`prover`** (from `src/main.rs`): Main proof generation tool
- **`gen-app-id`** (from `src/bin/gen-app-id.rs`): Application ID generator

## Proof Generation

Generates a zero-knowledge proof using the compiled guest binary and inputs defined in `src/main.rs`.

### Usage

```bash
cargo run --release --bin prover
```

Output: Saves proof data to `target/pico_out/inputs.json`.

## Generate App ID

This tool generates the application ID from a given ELF file. This ID is required for registering the application with the Brevis network.

### Usage

```bash
cargo run --bin gen-app-id -- --elf <path_to_elf>
```

Example:
```bash
cargo run --bin gen-app-id -- --elf ../app/elf/riscv32im-pico-zkvm-elf
```

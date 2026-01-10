# Prover Tools

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

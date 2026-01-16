# Prover Tools

## Binary Structure

This workspace contains multiple binaries:

- **`prover`** (from `src/main.rs`): Long-running Prover Service that listens for proof requests via Google Cloud Pub/Sub.
- **`gen-app-id`** (from `src/bin/gen-app-id.rs`): Application ID generator.

## Prover Service

The Prover Service is a long-running application that subscribes to a Google Cloud Pub/Sub subscription, processes proof generation requests, and publishes the results to a specified topic.

### Configuration

The service is configured via environment variables or a `.env` file:

| Variable | Description | Default |
|----------|-------------|---------|
| `GCP_PROJECT_ID` | Google Cloud Project ID | (Required) |
| `PROVER_SUBSCRIPTION` | Pub/Sub subscription ID for incoming requests | (Required) |
| `RESULT_TOPIC` | Pub/Sub topic ID for publishing results | (Required) |
| `MAX_CONCURRENT_PROOFS` | Max concurrent proof generation tasks | `2` |
| `PROOF_TIMEOUT_SECS` | Timeout for a single proof generation (seconds) | `3600` |
| `ELF_PATH` | Path to the RISC-V ELF binary | `../app/elf/riscv32im-pico-zkvm-elf` |
| `OUTPUT_DIR` | Directory for storing proof artifacts | `data` |
| `LOG_LEVEL` | Logging level (info, debug, trace) | `info` |

### Usage

1. **Set up environment variables:**

   Create a `.env` file or export variables directly.

   ```bash
   export GCP_PROJECT_ID=your-project-id
   export PROVER_SUBSCRIPTION=prover-requests-sub
   export RESULT_TOPIC=prover-results
   ```

2. **Run the service:**

   ```bash
   cargo run --release --bin prover
   ```

   The service will start, load the ELF file, and begin listening for Pub/Sub messages.

### Local Development (with Emulator)

To run the service against a local Pub/Sub emulator:

```bash
export PUBSUB_EMULATOR_HOST=localhost:8085
export GCP_PROJECT_ID=test-project
export PROVER_SUBSCRIPTION=prover-requests-sub
export RESULT_TOPIC=prover-results

cargo run --release --bin prover
```

### Groth16 Trusted Setup Management

The service **automatically manages Groth16 trusted setup** based on existing setup files in the `OUTPUT_DIR` (default: `data/`):

- **First run or setup files missing**:
  - Automatically performs a new trusted setup.
  - Generates `vm_pk` (Proving Key) and `vm_vk` (Verification Key).
  - Logs: `Running trusted setup...`

- **Subsequent runs**:
  - Reuses existing `vm_pk` and `vm_vk` files.
  - Logs: `Reusing existing Groth16 setup`

**When to force a new setup:**

If you modify the circuit logic in `app/src/main.rs`, you **must** delete the old setup files before restarting the service:

```bash
rm ./data/vm_pk ./data/vm_vk
cargo run --release --bin prover
```

This regenerates the keys. Remember to also redeploy the verifier contract and update your configuration if the verification key changes.

## Generate App ID

This tool generates the `app_id` from a given ELF file: 

```
ELF Binary → Program → (Proving Key, Verifying Key) → app_id = hash(VK)
```

1. **Compiles ELF to Program**: Converts the RISC-V ELF binary into a Pico program representation
2. **Generates Cryptographic Keys**: Creates both:
   - **Proving Key (PK)**: Used by the prover to generate proofs
   - **Verifying Key (VK)**: Used by the verifier to validate proofs
3. **Computes App ID**: Generates a unique application identifier by hashing the verification key using BN254 elliptic curve
   - Format: 64-character hex string (uint256 without `0x` prefix)
   - This `app_id` corresponds to the `riscvVkey` parameter in the smart contract's `verifyPicoProof` function

The `app_id` serves as a unique identifier that links your RISC-V program to its verification key on-chain, enabling trustless proof verification. It is also required for registering the application with the Brevis network.

### Usage

```bash
cargo run --bin gen-app-id -- --elf <path_to_elf>
```

Example:
```bash
cargo run --bin gen-app-id -- --elf ../app/elf/riscv32im-pico-zkvm-elf
```

Output:
```
Generated app_id: 0x<64_character_hex_string>
```

The current application's app-id is:
```
0x000b1cc7dd74154f7e00097b8bf7dc33719c4f8530e917d52358e4ce4ec21de4
```

# Prover Service

A long-running application that subscribes to a Google Cloud Pub/Sub subscription, processes proof generation requests, and publishes the results to a specified topic.

## Configuration

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

## Usage

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

## Local Development (with Emulator)

To run the service against a local Pub/Sub emulator:

```bash
export PUBSUB_EMULATOR_HOST=localhost:8085
export GCP_PROJECT_ID=test-project
export PROVER_SUBSCRIPTION=prover-requests-sub
export RESULT_TOPIC=prover-results

cargo run --release --bin prover
```

## Groth16 Setup Files

The service requires pre-generated Groth16 setup files in the `OUTPUT_DIR` (default: `data/`):

- `vm_pk` - Proving Key
- `vm_vk` - Verification Key

These files are generated during the one-time setup process (see main README Step 2). The service will fail to start if these files are missing.

If you modify the circuit logic in `app/src/main.rs`, you must regenerate the setup files and redeploy the verifier contract.

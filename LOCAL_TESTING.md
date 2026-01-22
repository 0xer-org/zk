# Test Proof Generation with Pub/Sub Emulator

#### Terminal 1: Start Emulator

```bash
docker run -d --rm --name pubsub-emulator -p 8085:8085 \
  -e PUBSUB_PROJECT_ID=test-project \
  gcr.io/google.com/cloudsdktool/google-cloud-cli:emulators \
  gcloud beta emulators pubsub start --project=test-project --host-port=0.0.0.0:8085
```

Note: To stop the emulator manually:

```bash
docker stop pubsub-emulator && docker rm pubsub-emulator
```

#### Terminal 2: Setup and Listen

```bash
export PUBSUB_EMULATOR_HOST=localhost:8085
npm run pubsub:setup
npm run pubsub:listen
```

#### Terminal 3: Run Prover (Docker)

Environment Variables:

| Variable                | Description               | Default    |
| ----------------------- | ------------------------- | ---------- |
| `GCP_PROJECT_ID`        | Google Cloud project ID   | (required) |
| `PROVER_SUBSCRIPTION`   | Subscription for requests | (required) |
| `RESULT_TOPIC`          | Topic for results         | (required) |
| `MAX_CONCURRENT_PROOFS` | Concurrent proof limit    | 2          |
| `PROOF_TIMEOUT_SECS`    | Timeout per proof         | 3600       |

Note: Messages are ACKed immediately upon receipt to prevent redelivery during long proof generation. If proof generation fails, the request will NOT be automatically retried. The caller should handle retries based on the error response.

##### Build Docker Image (first time only)

```bash
docker build -t prover-service:local .
```

##### Run Prover Service

The prover uses Docker-in-Docker for Groth16 proof generation. The inner container (`pico_gnark_cli`) is spawned by the **host's** Docker daemon, so it needs to access files on the **host** filesystem. We must use the same absolute path (e.g., `/Users/user_name/Projects/Twin3/zk/prover/data`) on both host and container.

**macOS:**

```bash
docker run --rm \
  --name prover-service \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v [ABSOLUTE_PATH_TO_DATA]:[ABSOLUTE_PATH_TO_DATA] \
  -e PUBSUB_EMULATOR_HOST=host.docker.internal:8085 \
  -e GCP_PROJECT_ID=test-project \
  -e PROVER_SUBSCRIPTION=prover-requests-sub \
  -e RESULT_TOPIC=prover-results \
  -e OUTPUT_DIR=[ABSOLUTE_PATH_TO_DATA] \
  -e ELF_PATH=/app/app/elf/riscv32im-pico-zkvm-elf \
  -e MAX_CONCURRENT_PROOFS=2 \
  -e RUST_LOG=info \
  prover-service:local
```

**Linux:**

```bash
docker run --rm \
  --name prover-service \
  --network host \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v [ABSOLUTE_PATH_TO_DATA]:[ABSOLUTE_PATH_TO_DATA] \
  -e PUBSUB_EMULATOR_HOST=localhost:8085 \
  -e GCP_PROJECT_ID=test-project \
  -e PROVER_SUBSCRIPTION=prover-requests-sub \
  -e RESULT_TOPIC=prover-results \
  -e OUTPUT_DIR=[ABSOLUTE_PATH_TO_DATA] \
  -e ELF_PATH=/app/app/elf/riscv32im-pico-zkvm-elf \
  -e MAX_CONCURRENT_PROOFS=2 \
  -e RUST_LOG=info \
  prover-service:local
```

#### Terminal 4: Send Test Messages

```bash
export PUBSUB_EMULATOR_HOST=localhost:8085
npm run pubsub:publish normal         # Standard test (0.75 recaptcha, SMS & bio verified)
# or
npm run pubsub:publish boundary       # Edge case (perfect recaptcha 1.0, all verified)
# or
npm run pubsub:publish invalid_json   # Malformed JSON for error handling
# or
npm run pubsub:publish missing_fields # Missing `bio_verified` field
```

### Step 5: Verify Proof On-Chain

After generating a proof, verify it on-chain using the deployed verifier contract.

**Note**: The prover generates temporary files (e.g., `groth16-proof.json`, `Groth16Verifier.sol`) in a request-specific directory during proof generation. These files are automatically deleted after the proof data is read and sent via Pub/Sub. When `npm run pubsub:listen` receives a successful proof, it saves the proof to `prover/data/groth16-proof.json` in the format required for on-chain verification.

To verify the proof on chain, set the `NETWORK` environment variable:

```bash
# Ethereum Mainnet
NETWORK=mainnet npm run verify

# Ethereum Sepolia (default)
NETWORK=sepolia npm run verify

# BSC Mainnet
NETWORK=bsc npm run verify

# BSC Testnet
NETWORK=bsc-testnet npm run verify
```

The script loads proof data from `prover/data/groth16-proof.json` (auto-saved by `pubsub:listen`) and calls `verifyPicoProof()` on the deployed PicoVerifier contract.

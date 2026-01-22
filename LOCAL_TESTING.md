# Local Testing Guide

## Prerequisites

Before getting started, ensure you have the following installed:

### 1. Docker (for proof generation)

Docker is required for:

- Generating the Groth16 setup files and verifier contract (Step 2)
- Generating proofs via Pub/Sub prover service (Step 4)

Install Docker from [docker.com](https://www.docker.com/get-started)

### 2. Pico zkVM

Follow the [Pico installation instructions](https://pico-docs.brevis.network/getting-started/installation.html)

### 3. Foundry (for smart contract deployment)

Install Foundry:

```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

Install Foundry dependencies:

```bash
forge install foundry-rs/forge-std --no-git
```

### 4. Node.js (for on-chain verification)

Node.js v18 or later is recommended.

Install dependencies:

```bash
npm install
```

### 5. Environment Configuration

Copy the example environment file:

```bash
cp .env.example .env
```

Edit `.env` and configure:

- `PRIVATE_KEY`: Your wallet's private key (with 0x prefix) - for deployment
- RPC URLs for networks you want to use:
  - `SEPOLIA_RPC_URL`: Ethereum Sepolia testnet
  - `MAINNET_RPC_URL`: Ethereum Mainnet
  - `BSC_TESTNET_RPC_URL`: BSC Testnet
  - `BSC_RPC_URL`: BSC Mainnet
- `ETHERSCAN_API_KEY`: (Optional) For contract verification (Etherscan v2 supports all networks)
- Verifier contract addresses (fill in after deployment):
  - `SEPOLIA_VERIFIER`, `BSC_TESTNET_VERIFIER`, `BSC_VERIFIER`, `MAINNET_VERIFIER`

## One-Time Setup

These steps only need to be performed once when setting up the project or when you modify the circuit logic.

### Step 1: Build the Guest Program (Circuit Compilation)

From the `app/` directory, build the ZKP program:

```bash
cd app
cargo pico build
```

This compiles the guest program to a RISC-V ELF binary at `app/elf/riscv32im-pico-zkvm-elf`.

**Note**: You only need to rebuild the guest program if you modify the circuit logic in `app/src/main.rs`. For different input values, you don't need to rebuild.

### Step 2: Generate Groth16 Setup Files and Verifier Contract

Generate the Groth16 proving key, verification key, and `Groth16Verifier.sol` contract by running the setup script.

From the project root directory:

```bash
cargo run --release --bin setup
```

This command:

- Runs the full proof generation pipeline (RISCV → RECURSION → EVM phases) with dummy inputs
- Generates Groth16 ProvingKey (`vm_pk`) and VerificationKey (`vm_vk`)
- Outputs `Groth16Verifier.sol` and other artifacts to `prover/data/`

**Note**: This setup only needs to be run once. The generated `vm_pk` and `vm_vk` files are reused for all subsequent proofs.

### Step 3: Deploy Verifier Contract

Deploy the Solidity verifier contract to verify proofs on-chain. You only need to deploy once per network.

Copy the `Groth16Verifier.sol` to the contracts source directory:

```bash
cp prover/data/Groth16Verifier.sol contracts/src/
```

Then, navigate to the contracts directory and deploy using Foundry:

```bash
cd contracts
export $(grep -v '^#' ../.env | xargs)
forge script script/Deploy.s.sol:DeployPicoVerifier \
    --rpc-url $RPC_URL \
    --broadcast \
    --verify \
    -vvvv
```

**Note**: Replace `$RPC_URL` with the appropriate RPC URL environment variable for your target network:

- `$SEPOLIA_RPC_URL` for Ethereum Sepolia
- `$MAINNET_RPC_URL` for Ethereum Mainnet
- `$BSC_TESTNET_RPC_URL` for BSC Testnet
- `$BSC_RPC_URL` for BSC Mainnet

Make sure your `.env` file contains `PRIVATE_KEY` and the appropriate RPC URL.

After the deployment, save the deployed contract address to your `.env` file:

```bash
# Add to .env
SEPOLIA_VERIFIER=0x...        # For Sepolia deployment
BSC_TESTNET_VERIFIER=0x...    # For BSC Testnet deployment
# etc.
```

**TODO**: Verifying the on-chain contract, which requires a command that verifies the relation between the `vm_pk` and the ELF.

### Step 4: Test Proof Generation with Pub/Sub Emulator

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

##### Prepare Host Directory (first time only)

The prover uses Docker-in-Docker for Groth16 proof generation. The inner container (`pico_gnark_cli`) is spawned by the **host's** Docker daemon, so it needs to access files on the **host** filesystem. We must use the same absolute path (`/app/data`) on both host and container.

```bash
# Create host data directory (same path used inside container)
sudo mkdir -p /app/data

# Copy Groth16 setup files from project to host directory
sudo cp prover/data/vm_pk prover/data/vm_vk /app/data/

# Verify files are copied (vm_pk ~1.3GB, vm_vk ~520 bytes)
ls -la /app/data/
```

##### Run Prover Service

**macOS:**

```bash
docker run --rm \
  --name prover-service \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v /app/data:/app/data \
  -e PUBSUB_EMULATOR_HOST=host.docker.internal:8085 \
  -e GCP_PROJECT_ID=test-project \
  -e PROVER_SUBSCRIPTION=prover-requests-sub \
  -e RESULT_TOPIC=prover-results \
  -e OUTPUT_DIR=/app/data \
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
  -v /app/data:/app/data \
  -e PUBSUB_EMULATOR_HOST=localhost:8085 \
  -e GCP_PROJECT_ID=test-project \
  -e PROVER_SUBSCRIPTION=prover-requests-sub \
  -e RESULT_TOPIC=prover-results \
  -e OUTPUT_DIR=/app/data \
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

The script:

1. Loads proof data from `prover/data/groth16-proof.json` (auto-saved by `pubsub:listen`)
2. Reads the network from `NETWORK` environment variable (defaults to `sepolia`)
3. Connects to the blockchain via your configured RPC URL
4. Calls `verifyPicoProof()` on the deployed PicoVerifier contract
5. Reports whether the verification succeeded or failed

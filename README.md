# Human Index ZKP with Pico

A complete zero-knowledge proof system for calculating and verifying a human verification index, from proof generation to on-chain verification using the Pico zkVM framework.

## Overview

This project implements a full ZKP pipeline that:
1. **Compiles the ZKP circuit** (one-time setup)
2. **Generates the verifier contract** directly from the circuit (one-time setup)
3. **Deploys a Solidity verifier** contract to Ethereum and BSC networks (one-time setup)
4. **Runs a Pub/Sub prover service** that receives proof requests and returns results via Google Cloud Pub/Sub
5. **Generates proofs** of correct human index calculation without revealing private verification data (repeatable)
6. **Verifies proofs on-chain** using the deployed contract (repeatable)

## Pub/Sub Prover Service

The prover can run as a cloud-native microservice using Google Cloud Pub/Sub:

```
┌─────────────────┐
│   Pub/Sub       │
│   Topic         │
│  (Requests)     │
└────────┬────────┘
         │
         │ Subscribe
         ▼
┌─────────────────────────────────────────┐
│        Prover Service                   │
│  ┌───────────────────────────────┐     │
│  │  Subscription Handler         │     │
│  │  - Receives messages          │     │
│  │  - Semaphore (2-4 concurrent) │     │
│  │  - ACK/NACK logic             │     │
│  └───────────┬───────────────────┘     │
│              │                          │
│              ▼                          │
│  ┌───────────────────────────────┐     │
│  │  Proof Generator              │     │
│  │  - Cached ELF                 │     │
│  │  - Blocking proof generation  │     │
│  │  - Base64 encoding            │     │
│  └───────────┬───────────────────┘     │
│              │                          │
│              ▼                          │
│  ┌───────────────────────────────┐     │
│  │  Result Publisher             │     │
│  │  - Publishes to result topic  │     │
│  │  - Includes metrics           │     │
│  └───────────────────────────────┘     │
└─────────────────────────────────────────┘
         │
         │ Publish
         ▼
┌─────────────────┐
│   Pub/Sub       │
│   Topic         │
│   (Results)     │
└─────────────────┘
```

### Formula

```
humanIndex = floor((W1 + W2 * recaptchaScore + W3 * smsVerified + W4 * bioVerified) * 255)
```

### Privacy Model

- **Private Inputs** (hidden in the proof):
  - `recaptchaScore`: Score from reCAPTCHA verification (0.0 to 1.0)
  - `smsVerified`: Whether SMS verification passed (0 or 1)
  - `bioVerified`: Whether biometric verification passed (0 or 1)

- **Public Inputs** (committed to the proof):
  - `W1`, `W2`, `W3`, `W4`: Weight parameters for the calculation
  - `expected_output`: The computed human index value

## Prerequisites

Before getting started, ensure you have the following installed:

### 1. Docker (for verifier generation and proof generation)
Docker is required for:
- Generating the `Groth16Verifier.sol` contract (Step 3)
- Generating proofs (Step 6)

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

### Step 2: Generate Groth16Verifier Contract

You can generate the `Groth16Verifier.sol` contract directly using the Pico Gnark CLI Docker image, **without needing to generate a proof first**. This approach avoids the SDK limitation mentioned in [pico#93](https://github.com/brevis-network/pico/issues/93).

From the project root directory, run:

```bash
docker run --rm -v $(pwd)/prover/data:/data brevishub/pico_gnark_cli:1.2 \
  /pico_gnark_cli \
  -field "kb" \
  -cmd setup \
  -sol /data/Groth16Verifier.sol
```

This command:
- Mounts `/prover/data` (where `groth16_witness.json` lives) to `/data` in the container
- Generates `Groth16Verifier.sol` directly in `/data` (which maps to `/prover/data` on your host)

### Step 3: Deploy Verifier Contract

Deploy the Solidity verifier contract to verify proofs on-chain. You only need to deploy once per network.

Copy the generated `Groth16Verifier.sol` to the contracts source directory:

```bash
cp prover/data/Groth16Verifier.sol contracts/src/
```

Then, navigate to the contracts directory and deploy using Foundry:

```bash
cd contracts

# Load environment variables from .env file
set -a
source ../.env
set +a

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

**Important**: Save the deployed contract address to your `.env` file:
```bash
# Add to .env
SEPOLIA_VERIFIER=0x...        # For Sepolia deployment
BSC_TESTNET_VERIFIER=0x...    # For BSC Testnet deployment
# etc.
```

### Step 4: Test Proof Generation with Pub/Sub Emulator

**Terminal 1: Start Emulator**
```bash
docker rm -f pubsub-emulator 2>/dev/null
docker run -d --name pubsub-emulator -p 8085:8085 \
  -e PUBSUB_PROJECT_ID=test-project \
  gcr.io/google.com/cloudsdktool/google-cloud-cli:emulators \
  gcloud beta emulators pubsub start --project=test-project --host-port=0.0.0.0:8085
```

Note: To stop the emulator:

```bash
docker stop pubsub-emulator && docker rm pubsub-emulator`
```

**Terminal 2: Setup and Listen**
```bash
export PUBSUB_EMULATOR_HOST=localhost:8085
npm run pubsub:setup
npm run pubsub:listen
```

**Terminal 3: Run Prover**

Environment Variables:

| Variable                | Description               | Default    |
| ----------------------- | ------------------------- | ---------- |
| `GCP_PROJECT_ID`        | Google Cloud project ID   | (required) |
| `PROVER_SUBSCRIPTION`   | Subscription for requests | (required) |
| `RESULT_TOPIC`          | Topic for results         | (required) |
| `MAX_CONCURRENT_PROOFS` | Concurrent proof limit    | 2          |
| `PROOF_TIMEOUT_SECS`    | Timeout per proof         | 3600       |

Note: Messages are ACKed immediately upon receipt to prevent redelivery during long proof generation. If proof generation fails, the request will NOT be automatically retried. The caller should handle retries based on the error response.

```bash
cargo run --release --bin prover
```

**Terminal 4: Send Test Messages**
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

When `npm run pubsub:listen` receives a successful proof, it automatically saves the proof to `prover/data/inputs.json` in the format required for on-chain verification.

By default, the script verifies on Ethereum Sepolia:

```bash
npm run verify
```

To verify on a different network, set the `NETWORK` environment variable:

```bash
# Ethereum Mainnet
NETWORK=mainnet npm run verify

# Ethereum Sepolia
NETWORK=sepolia npm run verify

# BSC Mainnet
NETWORK=bsc npm run verify

# BSC Testnet
NETWORK=bsc-testnet npm run verify
```

The script:
1. Loads proof data from `prover/data/inputs.json` (auto-saved by `pubsub:listen`)
2. Reads the network from `NETWORK` environment variable (defaults to `sepolia`)
3. Connects to the blockchain via your configured RPC URL
4. Calls `verifyPicoProof()` on the deployed PicoVerifier contract
5. Reports whether the verification succeeded or failed

## Verifying the On-Chain Contract

This requires a command that verifies the relation between the `vm_pk` and the ELF.

### Deployed Contracts
- Sepolia: 0x6D30a5BE6A1b79Fd3D254b0cC3152f77731c2768 (verified on Etherscan)

## References

- [Pico Documentation](https://pico-docs.brevis.network/)
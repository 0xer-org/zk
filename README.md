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

```bash
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

## Setup

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

### Step 4: Run Pub/Sub Prover Service and Generate Proofs

To deploy the Pub/Sub prover service to GCP and test it, follow the instructions in [DEPLOY_GCP.md](DEPLOY_GCP.md).
Alternatively, you can follow the instructions in [LOCAL_TESTING.md](LOCAL_TESTING.md) to run the prover service locally using the Pub/Sub emulator.

## References

- [Pico Documentation](https://pico-docs.brevis.network/)

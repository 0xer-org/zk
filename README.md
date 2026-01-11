# Human Index ZKP with Pico

A complete zero-knowledge proof system for calculating and verifying a human verification index, from proof generation to on-chain verification using the Pico zkVM framework.

## Overview

This project implements a full ZKP pipeline that:
1. **Compiles the ZKP circuit** (one-time setup)
2. **Generates the verifier contract** directly from the circuit (one-time setup)
3. **Deploys a Solidity verifier** contract to Ethereum and BSC networks (one-time setup)
4. **Customizes input values** (optional, for proof generation)
5. **Generates proofs** of correct human index calculation without revealing private verification data (repeatable)
6. **Verifies proofs on-chain** using the deployed contract (repeatable)

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

### Step 2: Generate Application ID

Generate the application ID from your compiled ELF binary. This ID uniquely identifies your circuit and corresponds to the `riscvVkey` parameter in the smart contract:

```bash
cd prover
cargo run --bin gen-app-id -- --elf ../app/elf/riscv32im-pico-zkvm-elf
```

This will output your `app_id`. For this project, the app_id is:
```
0x000b1cc7dd74154f7e00097b8bf7dc33719c4f8530e917d52358e4ce4ec21de4
```

### Step 3: Generate Groth16Verifier Contract

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

### Step 4: Deploy Verifier Contract

Deploy the Solidity verifier contract to verify proofs on-chain. You only need to deploy once per network.

The `Groth16Verifier.sol` generated in Step 3 is already in `contracts/src/`, so you can proceed directly to deployment.

Navigate to the contracts directory and deploy using Foundry:

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

---

## Generating and Verifying Proofs

After completing the one-time setup, you can generate and verify proofs with different input values. **These steps can be repeated as many times as needed using the same deployed verifier contract.**

### Step 5: Customize Input Values (Optional)

Edit `prover/src/main.rs` to modify the verification inputs for your proof:

#### Private Inputs

```rust
// recaptcha_score: 0.75 in fixed-point = 7500
let recaptcha_score = 7500u32;
// sms_verified: true = 1
let sms_verified = 1u32;
// bio_verified: true = 1
let bio_verified = 1u32;
```

#### Public Inputs

```rust
// W1 = 0.15 -> 1500
let w1 = 1500u32;
// W2 = 0.2 -> 2000
let w2 = 2000u32;
// W3 = 0.25 -> 2500
let w3 = 2500u32;
// W4 = 0.4 -> 4000
let w4 = 4000u32;
```

#### Fixed-Point Arithmetic

All decimal values use a **fixed-point scale of 10,000**:

- `0.15` → `1500` (multiply by 10,000)
- `0.75` → `7500` (multiply by 10,000)
- `1.0` → `10000` (multiply by 10,000)

To convert a decimal value to fixed-point:
```
fixed_point_value = decimal_value * 10000
```

### Step 6: Generate Proof

Run the prover to generate a ZKP proof with your input values:

```bash
cd prover
RUST_LOG=info cargo run --release --bin prover
```

This will:
1. Load the compiled guest program (from Step 1)
2. Use the input data you configured in Step 5 (or default values)
3. Generate a ZKP proof
4. Verify the proof locally
5. Save proof data to `target/pico_out/inputs.json`
6. Display the computation results

**Note**: This step creates a Docker container, which requires approximately 32GB of memory and can take around 30 minutes to complete.
If you encounter the following error, it likely indicates insufficient Docker memory:

```
thread 'main' panicked at prover/src/main.rs:69:69:
Failed to generate evm proof: the proof file is not exists in /Users/june/Desktop/zk/prover/../target/pico_out/proof.data. The preceding Docker step likely failed to write outputs (commonly due to insufficient Docker memory). Check the `docker` logs or increase Docker's memory limit.
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

### Step 7: Verify Proof On-Chain

After generating a proof, verify it on-chain using the deployed verifier contract.

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
1. Loads proof data from `target/pico_out/inputs.json` (generated in Step 6)
2. Reads the network from `NETWORK` environment variable (defaults to `sepolia`)
3. Connects to the blockchain via your configured RPC URL
4. Calls `verifyPicoProof()` on the deployed PicoVerifier contract
5. Reports whether the verification succeeded or failed

---

## Complete Workflow Examples

### Ethereum Sepolia

```bash
# === ONE-TIME SETUP ===
# Step 1: Build the circuit
cd app && cargo pico build && cd ..

# Step 2: Generate app ID
cd prover && cargo run --bin gen-app-id -- --elf ../app/elf/riscv32im-pico-zkvm-elf && cd ..
# app_id: 0x000b1cc7dd74154f7e00097b8bf7dc33719c4f8530e917d52358e4ce4ec21de4

# Step 3: Generate Groth16Verifier.sol directly (no proof needed!)
docker run --rm -v $(pwd)/prover/data:/data brevishub/pico_gnark_cli:1.2 \
  /pico_gnark_cli \
  -field "kb" \
  -cmd setup \
  -sol /data/Groth16Verifier.sol

# Step 4: Deploy verifier to Sepolia
cd contracts && \
set -a && source ../.env && set +a && \
forge script script/Deploy.s.sol:DeployPicoVerifier \
    --rpc-url $SEPOLIA_RPC_URL \
    --broadcast \
    --verify \
    -vvvv && \
cd ..
# Save the contract address to .env: SEPOLIA_VERIFIER=0x...

# === REPEATED OPERATIONS ===
# Step 5: (Optional) Edit input values in prover/src/main.rs

# Step 6: Generate proof
cd prover && RUST_LOG=info cargo run --release --bin prover && cd ..

# Step 7: Verify proof on-chain
NETWORK=sepolia npm run verify
```

## Verifying the On-Chain Contract

The deployed verifier contract can be verified against this repository to ensure the on-chain contract is bound to the circuit in this repo.

### What's Committed to This Repository
- Contract Source: `contracts/src/Groth16Verifier.sol` - the verifier contract code
- Verification Key: `prover/data/vm_vk` and `prover/data/vm_vk` - used to generate the contract
- Circuit Code: `app/src/main.rs` - the ZKP circuit logic

### How to Verify
1. Clone this repository
2. Follow Step 1 to build the guest program to get the ELF binary
4. Follow Step 6 to generate a proof
5. Follow Step 7 to verify the proof on-chain

If the proof verification passes on-chain, it proves that the on-chain Verifier contract is bound to the circuit in this repository.

### Deployed Contracts
- Sepolia: 0x8b04bc19292DcEf6B9EE3F8037B96A763351a77d (verified on Etherscan)

## References

- [Pico Documentation](https://pico-docs.brevis.network/)
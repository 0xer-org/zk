# Human Index ZKP with Pico

A complete zero-knowledge proof system for calculating and verifying a human verification index, from proof generation to on-chain verification using the Pico zkVM framework.

## Overview

This project implements a full ZKP pipeline that:
1. **Generates proofs** of correct human index calculation without revealing private verification data
2. **Deploys a Solidity verifier** contract to Ethereum and BSC networks
3. **Verifies proofs on-chain** using the deployed contract

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

## Project Structure

```
.
├── app/                      # ZKP guest program (runs in zkVM)
│   ├── src/main.rs           # Main proof logic
│   └── elf/                  # Compiled RISC-V binary
├── lib/                      # Shared library
│   └── src/lib.rs            # Data structures and calculation function
├── prover/                   # Host program (generates proofs)
│   └── src/main.rs           # Prover client and verification
├── contracts/                # Smart contract project
│   ├── src/                  # Solidity contracts
│   │   ├── PicoVerifier.sol  # On-chain proof verifier
│   │   ├── IPicoVerifier.sol # Verifier interface
│   │   └── Groth16Verifier.sol # Groth16 verifier
│   ├── script/               # Foundry deployment scripts
│   │   └── Deploy.s.sol      # Contract deployment script
│   └── foundry.toml          # Foundry configuration
├── deploy.sh                 # Deployment automation script
├── verify-proof.ts           # TypeScript verification script
└── package.json              # Node.js dependencies
```

## How It Works

1. **Guest Program** (`app/src/main.rs`):
   - Reads private verification results from stdin
   - Reads public weights from stdin
   - Computes the human index using fixed-point arithmetic
   - Commits public inputs and output to the proof

2. **Prover** (`prover/src/main.rs`):
   - Loads the guest program ELF binary
   - Provides inputs via stdin
   - Generates a ZKP proof
   - Extracts and verifies public outputs

3. **Library** (`lib/src/lib.rs`):
   - Defines data structures for inputs/outputs
   - Implements the human index calculation
   - Shared between guest and prover for consistency

## Prerequisites

Before getting started, ensure you have the following installed:

### 1. Pico zkVM
Follow the [Pico installation instructions](https://pico-docs.brevis.network/getting-started/installation.html)

### 2. Foundry (for smart contract deployment)
```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

Install Foundry dependencies:
```bash
forge install foundry-rs/forge-std --no-git
```

### 3. Node.js (for on-chain verification)
Node.js v18 or later is recommended.

Install dependencies:
```bash
npm install
```

### 4. Environment Configuration
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

## Step 1: Generate ZKP Proof

### 1.1 Build the Guest Program

From the `app/` directory, build the ZKP program:

```bash
cd app
cargo pico build
```

This compiles the guest program to a RISC-V ELF binary at `app/elf/riscv32im-pico-zkvm-elf`.

### 1.2 Build and Run the Prover

From the project root, run the prover:

```bash
cd ../prover
RUST_LOG=info cargo run --release
```

This will:
1. Load the compiled guest program
2. Set up the input data
3. Generate a ZKP proof
4. Verify the proof locally
5. Save proof data to `target/pico_out/inputs.json`
6. Display the results

### 1.3 Customizing Inputs

Edit `prover/src/main.rs` to modify the test inputs:

#### Private Inputs (lines 16-22)

```rust
// recaptcha_score: 0.75 in fixed-point = 7500
let recaptcha_score = 7500u32;
// sms_verified: true = 1
let sms_verified = 1u32;
// bio_verified: true = 1
let bio_verified = 1u32;
```

#### Public Inputs (lines 30-37)

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

## Step 2: Deploy Verifier Contract

Once you've generated a proof, you can deploy the Solidity verifier contract to verify proofs on-chain.

### 2.1 Make Deployment Script Executable

```bash
chmod +x deploy.sh
```

### 2.2 Deploy the Contract

Deploy to your chosen network:

```bash
# Ethereum networks
./deploy.sh sepolia       # Ethereum Sepolia testnet
./deploy.sh mainnet       # Ethereum Mainnet

# BSC networks
./deploy.sh bsc-testnet   # BSC Testnet
./deploy.sh bsc           # BSC Mainnet
```

The script will:
- Deploy the PicoVerifier contract
- Automatically verify the contract on the block explorer
- Save deployment information in the broadcast folder
- Display the deployed contract address

### 2.3 Deployment Output

After successful deployment, you'll find:
- Deployment details in `contracts/broadcast/Deploy.s.sol/<network>/run-latest.json`
- Contract address and transaction info in the console output
- Verified contract on the block explorer (if verification succeeded)

## Step 3: Verify Proof On-Chain

After deploying the verifier contract, you can verify your generated proof on-chain.

### 3.1 Run the Verification Script

By default, the script verifies on BSC Testnet:

```bash
npm run verify
```

To verify on a different network, set the `NETWORK` environment variable:

```bash
# BSC Testnet (default)
npm run verify

# BSC Mainnet
NETWORK=bsc npm run verify

# Ethereum Sepolia
NETWORK=sepolia npm run verify

# Ethereum Mainnet
NETWORK=mainnet npm run verify
```

Make sure you have:
1. Deployed the verifier contract to that network
2. Added the contract address to `.env` (e.g., `BSC_TESTNET_VERIFIER=0x...`)
3. Configured the corresponding RPC URL in `.env`

### 3.2 How It Works

The script:
1. Loads proof data from `target/pico_out/inputs.json` (generated in Step 1)
2. Reads the network from `NETWORK` environment variable (defaults to `bsc-testnet`)
3. Connects to the blockchain via your configured RPC URL
4. Calls `verifyPicoProof()` on the deployed PicoVerifier contract
5. Reports whether the verification succeeded or failed

Since `verifyPicoProof()` is a view function, no gas is required and no wallet is needed.

### 3.3 Complete Workflow Examples

#### BSC Testnet (Default)

```bash
# Step 1: Generate proof
cd app && cargo pico build && cd ..
RUST_LOG=info cargo run --release --manifest-path prover/Cargo.toml

# Step 2: Deploy verifier to BSC Testnet (one-time setup)
./deploy.sh bsc-testnet

# Step 3: Add contract address to .env
# BSC_TESTNET_VERIFIER=0x... (from deployment output)

# Step 4: Verify proof on-chain
npm run verify
```

#### Ethereum Sepolia

```bash
# Step 1: Generate proof (same as above)
cd app && cargo pico build && cd ..
RUST_LOG=info cargo run --release --manifest-path prover/Cargo.toml

# Step 2: Deploy verifier to Sepolia (one-time setup)
./deploy.sh sepolia

# Step 3: Add contract address to .env
# SEPOLIA_VERIFIER=0x... (from deployment output)

# Step 4: Verify proof on-chain
NETWORK=sepolia npm run verify
```

## References

- [Pico Documentation](https://pico-docs.brevis.network/)


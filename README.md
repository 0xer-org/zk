# Human Index ZKP with Pico

A zero-knowledge proof implementation for calculating a human verification index using the Pico zkVM framework.

## Overview

This project implements a ZKP circuit that proves the correct calculation of a human index score based on multiple verification factors, without revealing the private verification results.

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
├── app/                   # ZKP guest program (runs in zkVM)
│   ├── src/main.rs        # Main proof logic
│   └── elf/               # Compiled RISC-V binary
├── lib/                   # Shared library
│   └── src/lib.rs         # Data structures and calculation function
├── prover/                # Host program (generates proofs)
│   └── src/main.rs        # Prover client and verification
└── README.md
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

Check [Pico installation instructions](https://pico-docs.brevis.network/getting-started/installation.html)

## Building the Project

### 1. Build the Guest Program

From the `app/` directory, build the ZKP program:

```bash
cd app
cargo pico build
```

This compiles the guest program to a RISC-V ELF binary at `app/elf/riscv32im-pico-zkvm-elf`.

### 2. Build and Run the Prover

From the project root, run the prover:

```bash
cd ../prover
RUST_LOG=info cargo run --release
```

This will:
1. Load the compiled guest program
2. Set up the input data
3. Generate a ZKP proof
4. Verify the proof
5. Display the results

## Customizing Inputs

Edit `prover/src/main.rs` to modify the test inputs:

### Private Inputs (lines 16-22)

```rust
// recaptcha_score: 0.75 in fixed-point = 7500
let recaptcha_score = 7500u32;
// sms_verified: true = 1
let sms_verified = 1u32;
// bio_verified: true = 1
let bio_verified = 1u32;
```

### Public Inputs (lines 30-37)

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

### Fixed-Point Arithmetic

All decimal values use a **fixed-point scale of 10,000**:

- `0.15` → `1500` (multiply by 10,000)
- `0.75` → `7500` (multiply by 10,000)
- `1.0` → `10000` (multiply by 10,000)

To convert a decimal value to fixed-point:
```
fixed_point_value = decimal_value * 10000
```

## Smart Contract Deployment

This project includes a Solidity verifier contract that can verify proofs on-chain.

### Prerequisites for Deployment

1. Install Foundry:
```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

2. Install Foundry dependencies:
```bash
forge install foundry-rs/forge-std --no-git
```

### Deployment Setup

1. Copy the example environment file:
```bash
cp .env.example .env
```

2. Edit `.env` and add your configuration:
   - `PRIVATE_KEY`: Your wallet's private key (with 0x prefix)
   - Network-specific RPC URLs as needed
   - `ETHERSCAN_API_KEY`: (Optional) For contract verification

3. Make the deployment script executable:
```bash
chmod +x deploy.sh
```

### Deploy the Contract

Deploy to Sepolia testnet:
```bash
./deploy.sh sepolia
```

Deploy to Mainnet:
```bash
./deploy.sh mainnet
```

The script will:
- Deploy the PicoVerifier contract
- Automatically verify the contract on the block explorer
- Save deployment information in the broadcast folder
- Display the deployed contract address

### Deployment Output

After successful deployment, you'll find:
- Deployment details in `broadcast/Deploy.s.sol/<network>/run-latest.json`
- Contract address and transaction info in the console output
- Verified contract on the block explorer (if verification succeeded)

## References

- [Pico Documentation](https://pico-docs.brevis.network/)

# Prover Tools

## Binary Structure

This workspace contains multiple binaries:

- **`prover`** (from `src/main.rs`): Main proof generation tool
- **`gen-app-id`** (from `src/bin/gen-app-id.rs`): Application ID generator

## Proof Generation

Generates a zero-knowledge proof using the compiled guest binary and inputs defined in `src/main.rs`.

### Usage

```bash
cargo run --release --bin prover
```

Output: Saves proof data to `target/pico_out/inputs.json`.

### Groth16 Trusted Setup Management

The prover **automatically manages Groth16 trusted setup** based on existing setup files:

- **First run or setup files missing**:
  - Automatically performs a new trusted setup
  - Generates `target/pico_out/vm_pk` (Proving Key) and `target/pico_out/vm_vk` (Verification Key)
  - Outputs: `⚙️  Groth16 setup files not found. Running trusted setup...`

- **Subsequent runs**:
  - Reuses existing `vm_pk` and `vm_vk` files
  - Maintains consistent Groth16 verification parameters (ALPHA, BETA, GAMMA, DELTA)
  - Outputs: `✓ Reusing existing Groth16 setup from: ...`

**When to force a new setup:**

If you modify the circuit logic in `app/src/main.rs`, you **must** delete the old setup files:

```bash
rm ../target/pico_out/vm_pk ../target/pico_out/vm_vk
cargo run --release --bin prover
```

This regenerates the `Groth16Verifier.sol` with the new circuit parameters to match your updated circuit. Without this step, proofs may fail verification due to parameter mismatch.
Remember to also redeploy the new verifier contract and update your `.env` file with the new address after regenerating the setup.

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

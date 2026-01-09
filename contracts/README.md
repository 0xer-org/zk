# Smart Contracts

This directory contains all Solidity smart contracts and Foundry configuration for the Human Index ZKP project.

## Structure

- `src/` - Solidity contracts
  - `PicoVerifier.sol` - Main verifier contract that verifies Pico ZKP proofs on-chain
  - `IPicoVerifier.sol` - Verifier interface
  - `Groth16Verifier.sol` - Groth16 verification implementation
- `script/` - Foundry deployment scripts
  - `Deploy.s.sol` - Deployment script for PicoVerifier
- `foundry.toml` - Foundry configuration for compilation and deployment

## Building

```bash
cd contracts
forge build
```

## Deploying

Use the deployment script from the project root:

```bash
./deploy.sh <network>
```

Networks: `mainnet`, `sepolia`, `bsc`, `bsc-testnet`

## Configuration

The `foundry.toml` file contains:
- Solidity compiler version and settings
- RPC endpoints for different networks
- Etherscan API configuration for contract verification
import { ethers } from 'ethers';
import { readFileSync } from 'fs';
import { config } from 'dotenv';

// Load environment variables from .env file
config();

// PicoVerifier ABI - only the function we need
const PICO_VERIFIER_ABI = [
  "function verifyPicoProof(bytes32 riscvVkey, bytes calldata publicValues, uint256[8] calldata proof) external view"
];

// Network configuration
const NETWORK = process.env.NETWORK || 'sepolia';

// Get verifier address based on network
const VERIFIER_ADDRESS = process.env[`${NETWORK.toUpperCase().replace(/-/g, '_')}_VERIFIER`];

if (!VERIFIER_ADDRESS) {
  console.error(`❌ Error: ${NETWORK.toUpperCase().replace(/-/g, '_')}_VERIFIER not found in .env file`);
  console.error('Please copy .env.example to .env and fill in your verifier contract address');
  process.exit(1);
}

// Get RPC URL based on network
const RPC_URL = process.env[`${NETWORK.toUpperCase().replace(/-/g, '_')}_RPC_URL`];

if (!RPC_URL) {
  console.error(`❌ Error: ${NETWORK.toUpperCase().replace(/-/g, '_')}_RPC_URL not found in .env file`);
  console.error('Please copy .env.example to .env and fill in your RPC URL');
  process.exit(1);
}

// Network display names
const NETWORK_NAMES: Record<string, string> = {
  'mainnet': 'Ethereum Mainnet',
  'sepolia': 'Ethereum Sepolia',
  'bsc': 'BSC Mainnet',
  'bsc-testnet': 'BSC Testnet'
};

const networkName = NETWORK_NAMES[NETWORK] || NETWORK;

async function main() {
  // Read the Groth16 proof from the JSON file (path from argument or default)
  const proofPath = process.argv[2];
  if (!proofPath) {
    console.error('❌ Error: No proof path specified');
    console.error('Usage: npm run verify <proof-path>');
    console.error('Example: npm run verify prover/data/proofs/test-normal-1737654321.json');
    process.exit(1);
  }
  const inputsData = JSON.parse(readFileSync(proofPath, 'utf-8'));

  console.log('📄 Loaded proof data from:', proofPath);
  console.log('📍 Contract address:', VERIFIER_ADDRESS);
  console.log('🔗 Network:', networkName, `(${NETWORK})\n`);

  // Extract data from inputs.json
  const riscvVkey = inputsData.riscvVKey;
  const publicValues = inputsData.publicValues;
  const proof = inputsData.proof;

  console.log('📊 Proof Data:');
  console.log('  riscvVKey:', riscvVkey);
  console.log('  publicValues:', publicValues);
  console.log('  proof:', proof);
  console.log();

  // Connect to the specified network
  const provider = new ethers.JsonRpcProvider(RPC_URL);

  // Create contract instance
  const contract = new ethers.Contract(
    VERIFIER_ADDRESS as string,
    PICO_VERIFIER_ABI,
    provider
  );

  try {
    console.log('🔍 Verifying proof on-chain...');

    // Call verifyPicoProof (it's a view function, so no gas needed)
    await contract.verifyPicoProof(riscvVkey, publicValues, proof);

    console.log('✅ Proof verification SUCCEEDED!');
    console.log(`🎉 Your zero-knowledge proof is valid on ${networkName}.`);
  } catch (error: any) {
    console.log('❌ Proof verification FAILED!');
    console.error('Error:', error.message);

    if (error.data) {
      console.error('Error data:', error.data);
    }

    process.exit(1);
  }
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});

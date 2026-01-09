import { ethers } from 'ethers';
import { readFileSync } from 'fs';
import { config } from 'dotenv';

// Load environment variables from .env file
config();

// PicoVerifier ABI - only the function we need
const PICO_VERIFIER_ABI = [
  "function verifyPicoProof(bytes32 riscvVkey, bytes calldata publicValues, uint256[8] calldata proof) external view"
];

// Contract address on Sepolia
const SEPOLIA_VERIFIER = process.env.SEPOLIA_VERIFIER;

if (!SEPOLIA_VERIFIER) {
  console.error('❌ Error: SEPOLIA_VERIFIER not found in .env file');
  console.error('Please copy .env.example to .env and fill in your verifier contract address');
  process.exit(1);
}

// Sepolia RPC endpoint from environment variable
const SEPOLIA_RPC_URL = process.env.SEPOLIA_RPC_URL;

if (!SEPOLIA_RPC_URL) {
  console.error('❌ Error: SEPOLIA_RPC_URL not found in .env file');
  console.error('Please copy .env.example to .env and fill in your RPC URL');
  process.exit(1);
}

async function main() {
  // Read the inputs from the JSON file
  const inputsPath = './target/pico_out/inputs.json';
  const inputsData = JSON.parse(readFileSync(inputsPath, 'utf-8'));

  console.log('📄 Loaded proof data from:', inputsPath);
  console.log('📍 Contract address:', SEPOLIA_VERIFIER);
  console.log('🔗 Network: Sepolia\n');

  // Extract data from inputs.json
  const riscvVkey = inputsData.riscvVKey;
  const publicValues = inputsData.publicValues;
  const proof = inputsData.proof;

  console.log('📊 Proof Data:');
  console.log('  riscvVKey:', riscvVkey);
  console.log('  publicValues:', publicValues);
  console.log('  proof:', proof);
  console.log();

  // Connect to Sepolia
  const provider = new ethers.JsonRpcProvider(SEPOLIA_RPC_URL);

  // Create contract instance
  const contract = new ethers.Contract(
    SEPOLIA_VERIFIER as string,
    PICO_VERIFIER_ABI,
    provider
  );

  try {
    console.log('🔍 Verifying proof on-chain...');

    // Call verifyPicoProof (it's a view function, so no gas needed)
    await contract.verifyPicoProof(riscvVkey, publicValues, proof);

    console.log('✅ Proof verification SUCCEEDED!');
    console.log('🎉 Your zero-knowledge proof is valid on Sepolia.');
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

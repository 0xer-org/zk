import { ethers } from 'ethers';
import { readFileSync } from 'fs';
import { config } from 'dotenv';

// Load environment variables from .env file
config();

// PicoVerifier ABI - only the function we need
const PICO_VERIFIER_ABI = [
  "function verifyPicoProof(bytes32 riscvVkey, bytes calldata publicValues, uint256[8] calldata proof) external view"
];

// Network display names
const NETWORK_NAMES: Record<string, string> = {
  'mainnet': 'Ethereum Mainnet',
  'sepolia': 'Ethereum Sepolia',
  'bsc': 'BSC Mainnet',
  'bsc-testnet': 'BSC Testnet'
};

export interface ProofInputs {
  riscvVKey: string;
  publicValues: string;
  proof: number[];
}

export interface VerifyOptions {
  network?: string;
  verbose?: boolean;
}

export interface VerifyResult {
  success: boolean;
  error?: string;
  network: string;
}

/**
 * Verify a proof on-chain using the deployed PicoVerifier contract
 * @param proofData - The proof data to verify
 * @param options - Verification options (network, verbose logging)
 * @returns Result of the verification
 */
export async function verifyProof(
  proofData: ProofInputs,
  options: VerifyOptions = {}
): Promise<VerifyResult> {
  const { network = 'sepolia', verbose = true } = options;

  // Get verifier address based on network
  const verifierAddress = process.env[`${network.toUpperCase().replace(/-/g, '_')}_VERIFIER`];

  if (!verifierAddress) {
    const error = `${network.toUpperCase().replace(/-/g, '_')}_VERIFIER not found in .env file`;
    if (verbose) {
      console.error(`❌ Error: ${error}`);
      console.error('Please copy .env.example to .env and fill in your verifier contract address');
    }
    return { success: false, error, network };
  }

  // Get RPC URL based on network
  const rpcUrl = process.env[`${network.toUpperCase().replace(/-/g, '_')}_RPC_URL`];

  if (!rpcUrl) {
    const error = `${network.toUpperCase().replace(/-/g, '_')}_RPC_URL not found in .env file`;
    if (verbose) {
      console.error(`❌ Error: ${error}`);
      console.error('Please copy .env.example to .env and fill in your RPC URL');
    }
    return { success: false, error, network };
  }

  const networkName = NETWORK_NAMES[network] || network;

  if (verbose) {
    console.log('📍 Contract address:', verifierAddress);
    console.log('🔗 Network:', networkName, `(${network})\n`);
    console.log('📊 Proof Data:');
    console.log('  riscvVKey:', proofData.riscvVKey);
    console.log('  publicValues:', proofData.publicValues);
    console.log('  proof:', proofData.proof);
    console.log();
  }

  // Connect to the specified network
  const provider = new ethers.JsonRpcProvider(rpcUrl);

  // Create contract instance
  const contract = new ethers.Contract(
    verifierAddress,
    PICO_VERIFIER_ABI,
    provider
  );

  try {
    if (verbose) {
      console.log('🔍 Verifying proof on-chain...');
    }

    // Call verifyPicoProof (it's a view function, so no gas needed)
    await contract.verifyPicoProof(
      proofData.riscvVKey,
      proofData.publicValues,
      proofData.proof
    );

    if (verbose) {
      console.log('✅ Proof verification SUCCEEDED!');
      console.log(`🎉 Your zero-knowledge proof is valid on ${networkName}.`);
    }

    return { success: true, network: networkName };
  } catch (error: any) {
    if (verbose) {
      console.log('❌ Proof verification FAILED!');
      console.error('Error:', error.message);
      if (error.data) {
        console.error('Error data:', error.data);
      }
    }

    return {
      success: false,
      error: error.message,
      network: networkName
    };
  }
}

/**
 * Load proof data from a file
 * @param requestId - Optional request ID. If not provided, loads the latest proof
 * @returns Proof data
 */
export function loadProofFromFile(requestId?: string): ProofInputs {
  let proofPath: string;

  if (requestId) {
    proofPath = `prover/data/proofs/${requestId}.json`;
  } else {
    proofPath = 'prover/data/groth16-proof.json';
  }

  const inputsData = JSON.parse(readFileSync(proofPath, 'utf-8'));
  return inputsData as ProofInputs;
}

/**
 * CLI entry point
 * Usage: npm run verify [request-id]
 */
async function main() {
  const args = process.argv.slice(2);
  const requestId = args.length > 0 ? args[0] : undefined;
  const network = process.env.NETWORK || 'sepolia';

  // Load proof from file
  const proofPath = requestId
    ? `prover/data/proofs/${requestId}.json`
    : 'prover/data/groth16-proof.json';

  console.log('📄 Loaded proof data from:', proofPath);

  const proofData = loadProofFromFile(requestId);

  // Verify proof on-chain
  const result = await verifyProof(proofData, { network, verbose: true });

  if (!result.success) {
    process.exit(1);
  }
}

// Only run main if this file is executed directly
if (require.main === module) {
  main().catch((error) => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}

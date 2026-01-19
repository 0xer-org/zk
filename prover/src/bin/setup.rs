// One-time Groth16 setup script
// Generates vm_pk, vm_vk, and Groth16Verifier.sol

use human_index_lib::{calculate_human_index, load_elf, HumanIndexPublicInputs, VerificationResults};
use pico_sdk::client::DefaultProverClient;
use std::path::PathBuf;

fn main() {
    println!("=== Pico Groth16 Setup ===\n");

    // Paths
    let elf_path = std::env::var("ELF_PATH")
        .unwrap_or_else(|_| "./app/elf/riscv32im-pico-zkvm-elf".to_string());
    let output_dir = std::env::var("OUTPUT_DIR")
        .unwrap_or_else(|_| "prover/data".to_string());

    let output_path = PathBuf::from(&output_dir)
        .canonicalize()
        .unwrap_or_else(|_| {
            std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");
            PathBuf::from(&output_dir)
                .canonicalize()
                .expect("Failed to resolve output directory")
        });

    println!("ELF path: {}", elf_path);
    println!("Output directory: {}\n", output_path.display());

    // Load ELF
    println!("Loading ELF...");
    let elf_data = load_elf(&elf_path);
    println!("ELF loaded ({} bytes)\n", elf_data.len());

    // Initialize prover client
    let client = DefaultProverClient::new(&elf_data);
    let mut stdin_builder = client.new_stdin_builder();

    // Use dummy test inputs for setup (the actual values don't matter for setup)
    let verification_results = VerificationResults {
        recaptcha_score: 75,
        sms_verified: 1,
        bio_verified: 1,
    };
    let public_inputs = HumanIndexPublicInputs {
        w1: 10,
        w2: 30,
        w3: 30,
        w4: 30,
        expected_output: 0, // Will be calculated
    };

    // Write private inputs
    stdin_builder.write(&verification_results.recaptcha_score);
    stdin_builder.write(&verification_results.sms_verified);
    stdin_builder.write(&verification_results.bio_verified);

    // Calculate expected output
    let expected_output = calculate_human_index(&verification_results, &public_inputs);
    println!("Test human index: {}\n", expected_output);

    // Write public inputs
    stdin_builder.write(&public_inputs.w1);
    stdin_builder.write(&public_inputs.w2);
    stdin_builder.write(&public_inputs.w3);
    stdin_builder.write(&public_inputs.w4);
    stdin_builder.write(&expected_output);

    // Run prove_evm with need_setup=true
    println!("Running Groth16 setup (this may take a while)...");
    println!("This will generate: vm_pk, vm_vk, Groth16Verifier.sol\n");

    client
        .prove_evm(stdin_builder, true, output_path.clone(), "kb")
        .expect("prove_evm with setup failed");

    println!("\n=== Setup Complete ===");
    println!("Generated files in {}:", output_path.display());
    println!("  - vm_pk (proving key)");
    println!("  - vm_vk (verification key)");
    println!("  - Groth16Verifier.sol (verifier contract)");
    println!("  - inputs.json (test proof data)");
    println!("\nNext step: Copy Groth16Verifier.sol to contracts/src/");
}

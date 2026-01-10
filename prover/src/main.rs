use human_index_lib::{calculate_human_index, load_elf, HumanIndexPublicInputs, VerificationResults};
use pico_sdk::{client::DefaultProverClient, init_logger};
use std::env;

fn main() {
    // Initialize logger
    init_logger();

    // Load the ELF file
    let elf = load_elf("../app/elf/riscv32im-pico-zkvm-elf");

    // Initialize the prover client
    let client = DefaultProverClient::new(&elf);
    // Initialize new stdin
    let mut stdin_builder = client.new_stdin_builder();

    // Set up private inputs (verification results)
    // recaptcha_score: 0.75 in fixed-point = 7500
    let recaptcha_score = 7500u32;
    // sms_verified: true = 1
    let sms_verified = 1u32;
    // bio_verified: true = 1
    let bio_verified = 1u32;

    // Write private inputs to stdin
    stdin_builder.write(&recaptcha_score);
    stdin_builder.write(&sms_verified);
    stdin_builder.write(&bio_verified);

    // Set up public inputs (weights in fixed-point)
    // W1 = 0.15 -> 1500
    let w1 = 1500u32;
    // W2 = 0.2 -> 2000
    let w2 = 2000u32;
    // W3 = 0.25 -> 2500
    let w3 = 2500u32;
    // W4 = 0.4 -> 4000
    let w4 = 4000u32;

    // Calculate expected output locally for verification
    let verification_results = VerificationResults {
        recaptcha_score,
        sms_verified,
        bio_verified,
    };
    let public_inputs_struct = HumanIndexPublicInputs {
        w1,
        w2,
        w3,
        w4,
        expected_output: 0, // Will be calculated
    };
    let expected_output = calculate_human_index(&verification_results, &public_inputs_struct);

    // Write public inputs to stdin
    stdin_builder.write(&w1);
    stdin_builder.write(&w2);
    stdin_builder.write(&w3);
    stdin_builder.write(&w4);
    stdin_builder.write(&expected_output);

    // Set up output path
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let output_dir = current_dir.join("data");
    // Create the output directory if it doesn't exist
    std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Check if Groth16 setup files already exist in data directory
    let vm_pk_path = output_dir.join("vm_pk");
    let vm_vk_path = output_dir.join("vm_vk");
    let need_setup = !vm_pk_path.exists() || !vm_vk_path.exists();

    // Generate EVM proof
    client.prove_evm(stdin_builder, need_setup, output_dir.clone(), "kb").expect("Failed to generate evm proof");
}
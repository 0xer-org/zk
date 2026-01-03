use fibonacci_lib::{calculate_human_index, load_elf, HumanIndexPublicInputs, VerificationResults};
use pico_sdk::{client::DefaultProverClient, init_logger};

fn main() {
    // Initialize logger
    init_logger();

    // Load the ELF file
    let elf = load_elf("app/elf/riscv32im-pico-zkvm-elf");

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

    // Generate proof
    let proof = client
        .prove_fast(stdin_builder)
        .expect("Failed to generate proof");

    // Decodes public values from the proof's public value stream.
    let public_buffer = proof.pv_stream.unwrap();

    // Deserialize public values
    // First, deserialize the HumanIndexPublicInputs
    let public_inputs: HumanIndexPublicInputs =
        bincode::deserialize(&public_buffer).expect("Failed to deserialize public inputs");

    // Then deserialize the computed output (the remaining bytes)
    let remaining_buffer = &public_buffer[bincode::serialized_size(&public_inputs).unwrap() as usize..];
    let computed_output: u32 =
        bincode::deserialize(remaining_buffer).expect("Failed to deserialize computed output");

    // Verify the public values
    verify_public_values(&verification_results, &public_inputs, computed_output, expected_output);
}

/// Verifies that the computed human index matches the expected value.
fn verify_public_values(
    verification_results: &VerificationResults,
    public_inputs: &HumanIndexPublicInputs,
    computed_output: u32,
    expected_output: u32,
) {
    println!("=== Human Index ZKP Verification ===");
    println!("\nPublic Inputs:");
    println!("  W1: {} (0.15)", public_inputs.w1);
    println!("  W2: {} (0.2)", public_inputs.w2);
    println!("  W3: {} (0.25)", public_inputs.w3);
    println!("  W4: {} (0.4)", public_inputs.w4);
    println!("  Expected Output: {}", public_inputs.expected_output);

    println!("\nPrivate Inputs (for verification only):");
    println!("  Recaptcha Score: {} (0.75)", verification_results.recaptcha_score);
    println!("  SMS Verified: {}", verification_results.sms_verified);
    println!("  Bio Verified: {}", verification_results.bio_verified);

    println!("\nComputed Output: {}", computed_output);
    println!("Expected Output: {}", expected_output);

    // Verify that the computed output matches the expected output
    assert_eq!(
        computed_output, expected_output,
        "Mismatch: computed output {} != expected output {}",
        computed_output, expected_output
    );

    println!("\n✓ Verification successful! The proof is valid.");
}

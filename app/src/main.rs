#![no_main]

pico_sdk::entrypoint!(main);
use fibonacci_lib::{calculate_human_index, HumanIndexPublicInputs, PublicValues, VerificationResults};
use pico_sdk::io::{commit, read_as};

pub fn main() {
    // Read private inputs (verification results) from the environment
    let recaptcha_score: u32 = read_as();
    let sms_verified: u32 = read_as();
    let bio_verified: u32 = read_as();

    let verification_results = VerificationResults {
        recaptcha_score,
        sms_verified,
        bio_verified,
    };

    // Read public inputs (weights and expected output) from the environment
    let w1: u32 = read_as();
    let w2: u32 = read_as();
    let w3: u32 = read_as();
    let w4: u32 = read_as();
    let expected_output: u32 = read_as();

    let public_inputs = HumanIndexPublicInputs {
        w1,
        w2,
        w3,
        w4,
        expected_output,
    };

    // Compute the human index
    let computed_output = calculate_human_index(&verification_results, &public_inputs);

    // Commit all public values as a single struct to the proof
    let public_values = PublicValues {
        inputs: public_inputs,
        computed_output,
    };
    commit(&public_values);
}

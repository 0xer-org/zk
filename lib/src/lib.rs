use serde::{Deserialize, Serialize};
use std::fs;

// Fixed-point scale factor for decimal precision (10,000 = 4 decimal places)
const SCALE: u32 = 10_000;

/// Public inputs for the human index calculation
#[repr(C)]
#[derive(Serialize, Deserialize, Debug)]
pub struct HumanIndexPublicInputs {
    pub w1: u32, // Weight 1 in fixed-point (e.g., 0.15 * 10000 = 1500)
    pub w2: u32, // Weight 2 in fixed-point (e.g., 0.2 * 10000 = 2000)
    pub w3: u32, // Weight 3 in fixed-point (e.g., 0.25 * 10000 = 2500)
    pub w4: u32, // Weight 4 in fixed-point (e.g., 0.4 * 10000 = 4000)
    pub expected_output: u32, // Expected human index result
}

/// All public values that are committed to the proof and can be verified
#[repr(C)]
#[derive(Serialize, Deserialize, Debug)]
pub struct PublicValues {
    pub inputs: HumanIndexPublicInputs,
    pub computed_output: u32,
}

/// Private inputs (verification results)
#[derive(Serialize, Deserialize, Debug)]
pub struct VerificationResults {
    pub recaptcha_score: u32, // In fixed-point (0 to 10000 for 0.0 to 1.0)
    pub sms_verified: u32,    // 0 or 1
    pub bio_verified: u32,    // 0 or 1
}

/// Calculates the human index using fixed-point arithmetic
///
/// Formula: floor((W1 + W2 * recaptchaScore + W3 * smsVerified + W4 * bioVerified) * 255)
///
/// All inputs are in fixed-point with SCALE = 10,000
pub fn calculate_human_index(
    verification_results: &VerificationResults,
    public_inputs: &HumanIndexPublicInputs,
) -> u32 {
    let recaptcha_score = verification_results.recaptcha_score;
    let sms_verified = verification_results.sms_verified;
    let bio_verified = verification_results.bio_verified;

    // Check if recaptcha_score > 0
    if recaptcha_score == 0 {
        return 0;
    }

    // Calculate sum in fixed-point arithmetic
    // sum = W1 + W2 * recaptchaScore + W3 * smsVerified + W4 * bioVerified
    let mut sum = public_inputs.w1;

    // W2 * recaptchaScore (both in fixed-point, so divide by SCALE)
    sum += (public_inputs.w2 * recaptcha_score) / SCALE;

    // W3 * smsVerified (sms_verified is 0 or 1, w3 is in fixed-point)
    sum += public_inputs.w3 * sms_verified;

    // W4 * bioVerified (bio_verified is 0 or 1, w4 is in fixed-point)
    sum += public_inputs.w4 * bio_verified;

    // Multiply by 255 and divide by SCALE to convert back from fixed-point
    // floor(sum * 255) where sum is in fixed-point
    (sum * 255) / SCALE
}

/// Loads an ELF file from the specified path.
pub fn load_elf(path: &str) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|err| {
        panic!("Failed to load ELF file from {}: {}", path, err);
    })
}
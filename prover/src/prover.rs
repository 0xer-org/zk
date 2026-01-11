use crate::error::ServiceError;
use crate::types::{ProofData, ProverRequest};
use human_index_lib::{calculate_human_index, load_elf};
use pico_sdk::client::DefaultProverClient;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Cached ELF data to avoid reloading for each proof
pub struct CachedElf {
    pub data: Vec<u8>,
}

impl CachedElf {
    /// Load and cache the ELF file once
    pub fn load(elf_path: &str) -> Result<Self, ServiceError> {
        let elf_data = load_elf(elf_path);
        Ok(Self { data: elf_data })
    }
}

/// Proof generator handles the core proving logic
pub struct ProofGenerator {
    cached_elf: Arc<CachedElf>,
    output_base_dir: PathBuf,
}

impl ProofGenerator {
    /// Create a new proof generator with cached ELF
    pub fn new(cached_elf: Arc<CachedElf>, output_base_dir: PathBuf) -> Self {
        Self {
            cached_elf,
            output_base_dir,
        }
    }

    /// Generate a proof for the given request
    /// This is a blocking operation and should be called via spawn_blocking
    pub fn generate_proof(
        &self,
        request: ProverRequest,
    ) -> Result<(ProofData, bool), ServiceError> {
        // Create request-specific output directory
        let output_dir = self.output_base_dir.join(&request.request_id);
        std::fs::create_dir_all(&output_dir).map_err(|e| {
            ServiceError::ProofGeneration(format!("Failed to create output directory: {}", e))
        })?;

        // Initialize the prover client with cached ELF
        let client = DefaultProverClient::new(&self.cached_elf.data);
        let mut stdin_builder = client.new_stdin_builder();

        // Write private inputs to stdin
        let verification_results = &request.verification_results;
        stdin_builder.write(&verification_results.recaptcha_score);
        stdin_builder.write(&verification_results.sms_verified);
        stdin_builder.write(&verification_results.bio_verified);

        // Calculate expected output
        let public_inputs = &request.public_inputs;
        let expected_output = calculate_human_index(verification_results, public_inputs);

        // Write public inputs to stdin
        stdin_builder.write(&public_inputs.w1);
        stdin_builder.write(&public_inputs.w2);
        stdin_builder.write(&public_inputs.w3);
        stdin_builder.write(&public_inputs.w4);
        stdin_builder.write(&expected_output);

        // Check if Groth16 setup files exist in the base data directory
        let vm_pk_path = self.output_base_dir.join("vm_pk");
        let vm_vk_path = self.output_base_dir.join("vm_vk");
        let need_setup = !vm_pk_path.exists() || !vm_vk_path.exists();

        // Generate EVM proof
        client
            .prove_evm(stdin_builder, need_setup, output_dir.clone(), "kb")
            .map_err(|e| {
                ServiceError::ProofGeneration(format!("prove_evm failed: {}", e))
            })?;

        // Read the generated proof files
        let proof_data = self.read_proof_files(&output_dir, expected_output)?;

        Ok((proof_data, need_setup))
    }

    /// Read and encode proof files to base64
    fn read_proof_files(
        &self,
        output_dir: &Path,
        human_index: u32,
    ) -> Result<ProofData, ServiceError> {
        // Read proof file
        let proof_path = output_dir.join("kb_proof.bin");
        let proof_bytes = std::fs::read(&proof_path).map_err(|e| {
            ServiceError::ProofGeneration(format!(
                "Failed to read proof file {}: {}",
                proof_path.display(),
                e
            ))
        })?;

        // Read public inputs file
        let public_inputs_path = output_dir.join("kb_public_inputs.bin");
        let public_inputs_bytes = std::fs::read(&public_inputs_path).map_err(|e| {
            ServiceError::ProofGeneration(format!(
                "Failed to read public inputs file {}: {}",
                public_inputs_path.display(),
                e
            ))
        })?;

        // Read verification key file
        let vk_path = output_dir.join("kb_vk.bin");
        let vk_bytes = std::fs::read(&vk_path).map_err(|e| {
            ServiceError::ProofGeneration(format!(
                "Failed to read verification key file {}: {}",
                vk_path.display(),
                e
            ))
        })?;

        // Encode to base64
        use base64::{engine::general_purpose::STANDARD, Engine};
        let proof = STANDARD.encode(&proof_bytes);
        let public_inputs = STANDARD.encode(&public_inputs_bytes);
        let verification_key = STANDARD.encode(&vk_bytes);

        Ok(ProofData {
            proof,
            public_inputs,
            verification_key,
            human_index,
        })
    }
}

/// Helper to load and cache ELF at service startup
pub async fn load_and_cache_elf(elf_path: &str) -> Result<Arc<CachedElf>, ServiceError> {
    // Load ELF in a blocking task since it's an IO operation
    let elf_path = elf_path.to_string();
    let cached_elf = tokio::task::spawn_blocking(move || CachedElf::load(&elf_path))
        .await
        .map_err(|e| ServiceError::ProofGeneration(format!("Failed to spawn ELF loading task: {}", e)))?
        .map_err(|e| ServiceError::ProofGeneration(format!("Failed to load ELF: {}", e)))?;

    Ok(Arc::new(cached_elf))
}
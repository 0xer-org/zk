use crate::error::ServiceError;
use crate::types::{ProofData, ProverRequest};
use human_index_lib::{calculate_human_index, load_elf};
use pico_sdk::client::DefaultProverClient;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

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
    ) -> Result<ProofData, ServiceError> {
        // Create request-specific output directory (must be absolute path for prove_evm)
        let output_dir = self
            .output_base_dir
            .join(&request.request_id)
            .canonicalize()
            .or_else(|_| {
                // If canonicalize fails (dir doesn't exist yet), create it first
                let dir = self.output_base_dir.join(&request.request_id);
                std::fs::create_dir_all(&dir)?;
                dir.canonicalize()
            })
            .map_err(|e| {
                ServiceError::ProofGeneration(format!("Failed to resolve output directory: {}", e))
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

        // Symlink setup files from base data directory to proof directory (avoid copying large files)
        let vm_pk_path = self.output_base_dir.join("vm_pk").canonicalize().map_err(|e| {
            ServiceError::ProofGeneration(format!(
                "Groth16 setup file vm_pk not found at {}. Run the setup command first. Error: {}",
                self.output_base_dir.join("vm_pk").display(),
                e
            ))
        })?;
        let vm_vk_path = self.output_base_dir.join("vm_vk").canonicalize().map_err(|e| {
            ServiceError::ProofGeneration(format!(
                "Groth16 setup file vm_vk not found at {}. Run the setup command first. Error: {}",
                self.output_base_dir.join("vm_vk").display(),
                e
            ))
        })?;

        let dest_vm_pk = output_dir.join("vm_pk");
        let dest_vm_vk = output_dir.join("vm_vk");
        std::os::unix::fs::symlink(&vm_pk_path, &dest_vm_pk).map_err(|e| {
            ServiceError::ProofGeneration(format!("Failed to symlink vm_pk: {}", e))
        })?;
        std::os::unix::fs::symlink(&vm_vk_path, &dest_vm_vk).map_err(|e| {
            ServiceError::ProofGeneration(format!("Failed to symlink vm_vk: {}", e))
        })?;

        // Generate EVM proof (never run trusted setup)
        let prove_result = client
            .prove_evm(stdin_builder, false, output_dir.clone(), "kb")
            .map_err(|e| {
                ServiceError::ProofGeneration(format!("prove_evm failed: {}", e))
            });

        // Read the generated proof files before cleanup
        let result = match prove_result {
            Ok(()) => self.read_proof_files(&output_dir, expected_output),
            Err(e) => Err(e),
        };

        // Always cleanup the request-specific output directory
        if let Err(e) = std::fs::remove_dir_all(&output_dir) {
            info!("Failed to cleanup output directory {}: {} (non-fatal)", output_dir.display(), e);
        }

        result
    }

    /// Read and encode proof files to base64
    fn read_proof_files(
        &self,
        output_dir: &Path,
        human_index: u32,
    ) -> Result<ProofData, ServiceError> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        use serde_json::Value;

        // Read inputs.json file generated by Pico SDK
        let inputs_path = output_dir.join("inputs.json");
        let inputs_content = std::fs::read_to_string(&inputs_path).map_err(|e| {
            ServiceError::ProofGeneration(format!(
                "Failed to read inputs file {}: {}. The preceding Docker step likely failed to write outputs (commonly due to insufficient Docker memory). Check the `docker` logs or increase Docker's memory limit.",
                inputs_path.display(),
                e
            ))
        })?;

        let inputs: Value = serde_json::from_str(&inputs_content).map_err(|e| {
            ServiceError::ProofGeneration(format!(
                "Failed to parse inputs.json: {}",
                e
            ))
        })?;

        // Extract proof array and encode to base64
        let proof_array = inputs.get("proof")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ServiceError::ProofGeneration("Missing proof array in inputs.json".to_string()))?;

        let proof_json = serde_json::to_string(&proof_array).map_err(|e| {
            ServiceError::ProofGeneration(format!("Failed to serialize proof: {}", e))
        })?;
        let proof = STANDARD.encode(proof_json.as_bytes());

        // Extract publicValues and encode to base64
        let public_values = inputs.get("publicValues")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::ProofGeneration("Missing publicValues in inputs.json".to_string()))?;
        let public_inputs = STANDARD.encode(public_values.as_bytes());

        // Extract riscvVKey and encode to base64
        let riscv_vkey = inputs.get("riscvVKey")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::ProofGeneration("Missing riscvVKey in inputs.json".to_string()))?;
        let verification_key = STANDARD.encode(riscv_vkey.as_bytes());

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
use crate::error::ServiceError;
use std::env;

/// Configuration for the prover service loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    /// GCP Project ID
    pub gcp_project_id: String,

    /// Subscription name to receive proof requests
    pub prover_subscription: String,

    /// Topic name to publish results
    pub result_topic: String,

    /// Maximum number of concurrent proof generations
    pub max_concurrent_proofs: usize,

    /// Timeout for each proof generation in seconds
    pub proof_timeout_secs: u64,

    /// Path to the ELF file
    pub elf_path: String,

    /// Output directory for proof artifacts
    pub output_dir: String,

    /// Whether to enable JSON logging
    pub json_logging: bool,

    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ServiceError> {
        dotenvy::dotenv().ok(); // Load .env file if it exists

        let gcp_project_id = env::var("GCP_PROJECT_ID")
            .map_err(|_| ServiceError::Config("GCP_PROJECT_ID not set".to_string()))?;

        let prover_subscription = env::var("PROVER_SUBSCRIPTION")
            .map_err(|_| ServiceError::Config("PROVER_SUBSCRIPTION not set".to_string()))?;

        let result_topic = env::var("RESULT_TOPIC")
            .map_err(|_| ServiceError::Config("RESULT_TOPIC not set".to_string()))?;

        let max_concurrent_proofs = env::var("MAX_CONCURRENT_PROOFS")
            .unwrap_or_else(|_| "2".to_string())
            .parse::<usize>()
            .map_err(|e| ServiceError::Config(format!("Invalid MAX_CONCURRENT_PROOFS: {}", e)))?;

        let proof_timeout_secs = env::var("PROOF_TIMEOUT_SECS")
            .unwrap_or_else(|_| "3600".to_string()) // Default 1 hour
            .parse::<u64>()
            .map_err(|e| ServiceError::Config(format!("Invalid PROOF_TIMEOUT_SECS: {}", e)))?;

        let elf_path = env::var("ELF_PATH")
            .unwrap_or_else(|_| "../app/elf/riscv32im-pico-zkvm-elf".to_string());

        // Default to prover/data relative to the cargo manifest directory
        let output_dir = env::var("OUTPUT_DIR")
            .unwrap_or_else(|_| {
                let manifest_dir = env!("CARGO_MANIFEST_DIR");
                format!("{}/data", manifest_dir)
            });

        let json_logging = env::var("JSON_LOGGING")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let log_level = env::var("LOG_LEVEL")
            .unwrap_or_else(|_| "info".to_string());

        Ok(Self {
            gcp_project_id,
            prover_subscription,
            result_topic,
            max_concurrent_proofs,
            proof_timeout_secs,
            elf_path,
            output_dir,
            json_logging,
            log_level,
        })
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ServiceError> {
        if self.max_concurrent_proofs == 0 {
            return Err(ServiceError::Config(
                "MAX_CONCURRENT_PROOFS must be greater than 0".to_string(),
            ));
        }

        if self.proof_timeout_secs == 0 {
            return Err(ServiceError::Config(
                "PROOF_TIMEOUT_SECS must be greater than 0".to_string(),
            ));
        }

        // Validate ELF file exists
        if !std::path::Path::new(&self.elf_path).exists() {
            return Err(ServiceError::Config(format!(
                "ELF file not found at: {}",
                self.elf_path
            )));
        }

        Ok(())
    }
}
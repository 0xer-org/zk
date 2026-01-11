use thiserror::Error;

/// Service-level errors for the prover
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Pub/Sub error: {0}")]
    PubSub(String),

    #[error("Proof generation failed: {0}")]
    ProofGeneration(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Shutdown signal received")]
    Shutdown,
}

impl ServiceError {
    pub fn error_type(&self) -> String {
        match self {
            ServiceError::PubSub(_) => "PubSubError",
            ServiceError::ProofGeneration(_) => "ProofGenerationError",
            ServiceError::Serialization(_) => "SerializationError",
            ServiceError::Io(_) => "IoError",
            ServiceError::Config(_) => "ConfigError",
            ServiceError::Timeout(_) => "TimeoutError",
            ServiceError::Shutdown => "ShutdownError",
        }
        .to_string()
    }
}
mod config;
mod error;
mod prover;
mod service;
mod types;

use config::Config;
use error::ServiceError;
use prover::load_and_cache_elf;
use service::ProverService;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    // Load configuration
    let config = Config::from_env()?;
    config.validate()?;

    // Initialize logging
    init_logging(&config);

    info!("Starting Pico ZK Prover Service");
    info!("Configuration loaded successfully");
    info!("  GCP Project: {}", config.gcp_project_id);
    info!("  Subscription: {}", config.prover_subscription);
    info!("  Result Topic: {}", config.result_topic);
    info!("  Max Concurrent Proofs: {}", config.max_concurrent_proofs);
    info!("  Proof Timeout: {}s", config.proof_timeout_secs);
    info!("  ELF Path: {}", config.elf_path);
    info!("  Output Dir: {}", config.output_dir);

    // Load and cache ELF file
    info!("Loading ELF file: {}", config.elf_path);
    let cached_elf = load_and_cache_elf(&config.elf_path).await?;
    info!("ELF file loaded and cached successfully");

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&config.output_dir)?;

    // Validate output directory is writable
    let test_file = format!("{}/.write_test", config.output_dir);
    std::fs::write(&test_file, "test").map_err(|e| {
        error!("Output directory not writable: {}", e);
        ServiceError::Config(format!(
            "Output directory '{}' is not writable: {}",
            config.output_dir, e
        ))
    })?;
    std::fs::remove_file(&test_file).ok();
    info!("Output directory ready: {}", config.output_dir);

    // Initialize prover service
    info!("Initializing Prover Service");
    let service = ProverService::new(config, cached_elf).await?;

    // Create cancellation token for graceful shutdown
    let cancellation_token = CancellationToken::new();
    let shutdown_token = cancellation_token.clone();

    // Spawn shutdown handler
    tokio::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            error!("Failed to listen for shutdown signal: {}", e);
        }
        info!("Shutdown signal received, stopping service...");
        shutdown_token.cancel();
    });

    // Run service with cancellation token
    match service.run(cancellation_token).await {
        Ok(_) => info!("Service stopped normally"),
        Err(e) => {
            error!("Service error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Initialize logging based on configuration
fn init_logging(config: &Config) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level));

    if config.json_logging {
        // JSON logging for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        // Human-readable logging for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
}

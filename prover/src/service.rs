use crate::config::Config;
use crate::error::ServiceError;
use crate::prover::{CachedElf, ProofGenerator};
use crate::types::{ProofError, ProofMetrics, ProverRequest, ProverResponse};
use chrono::Utc;
use google_cloud_googleapis::pubsub::v1::PubsubMessage;
use google_cloud_pubsub::client::{Client, ClientConfig};
use google_cloud_pubsub::subscription::Subscription;
use google_cloud_pubsub::topic::Topic;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

/// Prover service that subscribes to Pub/Sub and processes proof requests
pub struct ProverService {
    config: Config,
    cached_elf: Arc<CachedElf>,
    subscription: Subscription,
    result_publisher: Topic,
    semaphore: Arc<Semaphore>,
}

impl ProverService {
    /// Create a new prover service
    pub async fn new(config: Config, cached_elf: Arc<CachedElf>) -> Result<Self, ServiceError> {
        info!("Initializing Google Cloud Pub/Sub client");

        // Create Pub/Sub client
        let client_config = ClientConfig::default();

        let client = Client::new(client_config)
            .await
            .map_err(|e| ServiceError::PubSub(format!("Failed to create Pub/Sub client: {}", e)))?;

        // Get subscription with full path (required for emulator)
        let subscription_path = format!(
            "projects/{}/subscriptions/{}",
            config.gcp_project_id, config.prover_subscription
        );
        let subscription = client.subscription(&subscription_path);

        // Create persistent result topic publisher
        let result_topic_path = format!(
            "projects/{}/topics/{}",
            config.gcp_project_id, config.result_topic
        );
        let result_publisher = client.topic(&result_topic_path);

        // Create semaphore for concurrency control
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_proofs));

        info!(
            "Prover service initialized with max_concurrent_proofs={}",
            config.max_concurrent_proofs
        );

        Ok(Self {
            config,
            cached_elf,
            subscription,
            result_publisher,
            semaphore,
        })
    }

    /// Start the service and process messages
    pub async fn run(&self, cancellation_token: CancellationToken) -> Result<(), ServiceError> {
        info!(
            "Starting prover service, subscribing to '{}'",
            self.config.prover_subscription
        );

        let config = self.config.clone();
        let cached_elf = self.cached_elf.clone();
        let result_publisher = self.result_publisher.clone();
        let semaphore = self.semaphore.clone();

        // Subscribe to messages with handler function
        self.subscription
            .receive(
                move |message, _cancel| {
                    let config = config.clone();
                    let cached_elf = cached_elf.clone();
                    let result_publisher = result_publisher.clone();
                    let semaphore = semaphore.clone();

                    async move {
                        // Try to acquire permit
                        let permit = match semaphore.clone().try_acquire_owned() {
                            Ok(p) => p,
                            Err(_) => {
                                warn!("Too many concurrent proofs, nacking message");
                                if let Err(e) = message.nack().await {
                                    error!("Failed to NACK message: {}", e);
                                }
                                return;
                            }
                        };

                        let received_at = Utc::now();
                        let ack_id = message.ack_id().to_string();

                        // Immediately ACK to prevent redelivery (proof generation takes hours)
                        if let Err(e) = message.ack().await {
                            error!(ack_id = ack_id, "Failed to ACK message: {}", e);
                            drop(permit);
                            return;
                        }
                        debug!(ack_id = ack_id, "Message ACKed immediately to prevent redelivery");

                        // Process the message (no retry on failure)
                        match Self::process_message(
                            &message.message.data,
                            config,
                            cached_elf,
                            received_at,
                        )
                        .await
                        {
                            Ok(response) => {
                                // Publish result
                                if let Err(e) =
                                    Self::publish_result(&result_publisher, &response).await
                                {
                                    error!(
                                        request_id = response.request_id,
                                        "Failed to publish result: {}", e
                                    );
                                }
                            }
                            Err(e) => {
                                error!("Failed to process message: {}", e);
                                // Message already ACKed, no retry will happen
                            }
                        }

                        drop(permit);
                    }
                },
                cancellation_token,
                None,
            )
            .await
            .map_err(|e| ServiceError::PubSub(format!("Subscription receive error: {}", e)))?;

        Ok(())
    }

    /// Process a single message
    async fn process_message(
        data: &[u8],
        config: Config,
        cached_elf: Arc<CachedElf>,
        received_at: chrono::DateTime<Utc>,
    ) -> Result<ProverResponse, ServiceError> {
        // Parse request
        let request: ProverRequest = serde_json::from_slice(data)?;
        let request_id = request.request_id.clone();

        info!(request_id = %request_id, "Processing proof request");

        let started_at = Utc::now();

        // Create proof generator
        let output_dir = PathBuf::from(&config.output_dir);
        let generator = ProofGenerator::new(cached_elf, output_dir);

        // Generate proof with timeout
        let proof_timeout = Duration::from_secs(config.proof_timeout_secs);
        let request_clone = request.clone();

        let result = timeout(proof_timeout, async move {
            tokio::task::spawn_blocking(move || generator.generate_proof(request_clone))
                .await
                .map_err(|e| ServiceError::ProofGeneration(format!("Task join error: {}", e)))?
        })
        .await;

        let completed_at = Utc::now();
        let duration_ms = (completed_at - received_at).num_milliseconds() as u64;

        match result {
            Ok(Ok((proof_data, setup_required))) => {
                info!(
                    request_id = %request_id,
                    duration_ms = duration_ms,
                    setup_required = setup_required,
                    "Proof generated successfully"
                );

                let metrics = ProofMetrics {
                    received_at: received_at.to_rfc3339(),
                    started_at: started_at.to_rfc3339(),
                    completed_at: completed_at.to_rfc3339(),
                    duration_ms,
                    setup_required,
                };

                Ok(ProverResponse::success(request_id, proof_data, metrics))
            }
            Ok(Err(e)) => {
                error!(request_id = %request_id, "Proof generation failed: {}", e);

                let metrics = ProofMetrics {
                    received_at: received_at.to_rfc3339(),
                    started_at: started_at.to_rfc3339(),
                    completed_at: completed_at.to_rfc3339(),
                    duration_ms,
                    setup_required: false,
                };

                Ok(ProverResponse::failed(
                    request_id,
                    ProofError {
                        error_type: e.error_type(),
                        message: e.to_string(),
                        details: None,
                    },
                    Some(metrics),
                ))
            }
            Err(_) => {
                warn!(
                    request_id = %request_id,
                    timeout_secs = config.proof_timeout_secs,
                    "Proof generation timed out"
                );

                let metrics = ProofMetrics {
                    received_at: received_at.to_rfc3339(),
                    started_at: started_at.to_rfc3339(),
                    completed_at: completed_at.to_rfc3339(),
                    duration_ms,
                    setup_required: false,
                };

                Ok(ProverResponse::timeout(
                    request_id,
                    format!(
                        "Proof generation timed out after {} seconds",
                        config.proof_timeout_secs
                    ),
                    Some(metrics),
                ))
            }
        }
    }

    /// Publish result to result topic
    async fn publish_result(
        publisher: &Topic,
        response: &ProverResponse,
    ) -> Result<(), ServiceError> {
        let data = serde_json::to_vec(response)?;

        let message = PubsubMessage {
            data,
            ..Default::default()
        };

        let awaiter = publisher.new_publisher(None).publish(message).await;

        awaiter
            .get()
            .await
            .map_err(|e| ServiceError::PubSub(format!("Failed to get publish result: {}", e)))?;

        debug!(
            request_id = response.request_id,
            "Result published successfully"
        );

        Ok(())
    }
}

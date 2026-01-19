# Pub/Sub Ack Deadline Issue and Fix

## Problem

When running the prover service, a single request results in multiple proof generations for the same `request_id`.

**Observed behavior:**
- Sent 2 requests (`test-normal-1768771483` and `test-normal-1768771583`)
- Received 8 success results (4 for each request_id)
- Each result has different `proof` values but same `request_id`

## Root Cause

Pub/Sub **message redelivery** due to ack deadline expiration.

### How it happens:

1. Prover receives message and starts processing
2. ZK proof generation takes a long time (observed: 1-3.5 hours)
3. Pub/Sub ack deadline expires (default: 10 seconds)
4. Pub/Sub assumes message was not processed successfully
5. Pub/Sub redelivers the same message
6. Another worker (or same worker) receives and processes it again
7. Result: duplicate proof generations

### Evidence from metrics:

| request_id | received_at | duration_ms | setup_required |
|------------|-------------|-------------|----------------|
| test-normal-1768771483 | 21:24:43 | 12,622,058 (~3.5h) | true |
| test-normal-1768771483 | 00:55:05 | 3,624,112 (~1h) | false |
| test-normal-1768771483 | 01:55:29 | 3,679,625 (~1h) | false |
| test-normal-1768771483 | 07:26:30 | 1,109,543 (~18min) | false |

The same `request_id` was processed 4 times because Pub/Sub kept redelivering it.

## Current Code Issue

In `prover/src/service.rs`, the `receive()` call has no ack deadline configuration:

```rust
self.subscription
    .receive(
        move |message, _cancel| { ... },
        cancellation_token,
        None,  // <-- No ReceiveConfig, uses default ack deadline
    )
```

## Solution

### Option 1: Use ReceiveConfig (Quick Fix)

Add `ReceiveConfig` to set a longer ack deadline:

```rust
use google_cloud_pubsub::subscription::ReceiveConfig;

self.subscription
    .receive(
        move |message, _cancel| { ... },
        cancellation_token,
        Some(ReceiveConfig {
            stream_ack_deadline_seconds: 600,  // Max: 600 seconds (10 min)
            ..Default::default()
        }),
    )
```

**Limitation:** Max ack deadline is 600 seconds, but proof generation can take hours.

### Option 2: Heartbeat with ModifyAckDeadline (Recommended)

Implement a background task that periodically extends the ack deadline while processing:

```rust
async fn process_with_heartbeat(
    message: ReceivedMessage,
    subscription: Subscription,
    process_fn: impl Future<Output = Result<...>>,
) -> Result<...> {
    let ack_id = message.ack_id().to_string();

    // Spawn heartbeat task
    let heartbeat_handle = tokio::spawn({
        let subscription = subscription.clone();
        let ack_id = ack_id.clone();
        async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 min
            loop {
                interval.tick().await;
                // Extend deadline by 600 seconds
                if let Err(e) = subscription
                    .modify_ack_deadline(&[ack_id.clone()], 600)
                    .await
                {
                    error!("Failed to extend ack deadline: {}", e);
                    break;
                }
                debug!("Extended ack deadline for {}", ack_id);
            }
        }
    });

    // Process the message
    let result = process_fn.await;

    // Stop heartbeat
    heartbeat_handle.abort();

    result
}
```

### Option 3: Immediate Ack + Separate Status (Alternative)

1. Immediately ACK the message upon receipt
2. Track processing status in a separate store (Redis, database)
3. Publish results when complete
4. Handle duplicates via request_id deduplication

**Trade-off:** Loses Pub/Sub's automatic retry on failure.

## Recommended Implementation

Use **Option 2 (Heartbeat)** because:
- Maintains Pub/Sub's delivery guarantees
- Works with arbitrarily long processing times
- No external dependencies needed

## Additional Considerations

1. **Deduplication:** Even with the fix, consider adding request_id deduplication to handle edge cases

2. **Subscription Configuration:** Set subscription's default ack deadline to maximum (600s) via GCP Console or gcloud:
   ```bash
   gcloud pubsub subscriptions update prover-requests-sub \
       --ack-deadline=600
   ```

3. **Emulator Limitation:** The Pub/Sub emulator may not fully support `modify_ack_deadline`. Test with real GCP for production validation.

## Files to Modify

- `prover/src/service.rs` - Add heartbeat mechanism to `run()` method

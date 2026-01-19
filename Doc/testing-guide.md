# Prover Service Testing Guide

This guide covers how to test the ZK prover service locally using the Google Cloud Pub/Sub Emulator.

## Prerequisites

- Docker (for Pub/Sub Emulator)
- Node.js 18+ (or Python 3.7+)

## Setup

### Install Dependencies

**Node.js (recommended):**
```bash
npm install @google-cloud/pubsub
```

## Running Tests

**Terminal 1: Start Emulator**

```bash
docker rm -f pubsub-emulator 2>/dev/null
docker run -d \
    --name pubsub-emulator \
    -p 8085:8085 \
    -e PUBSUB_PROJECT_ID=test-project \
    gcr.io/google.com/cloudsdktool/google-cloud-cli:emulators \
    gcloud beta emulators pubsub start \
        --project=test-project \
        --host-port=0.0.0.0:8085
```

To stop the emulator:
```bash
docker stop pubsub-emulator && docker rm pubsub-emulator
```

**Terminal 2: Setup and Listen for Results**
```bash
export PUBSUB_EMULATOR_HOST=localhost:8085
npm run pubsub:setup
npm run pubsub:listen
```

**Terminal 3: Run Prover Service**
```bash
cd prover && cargo run --release --bin prover
```

**Terminal 4: Send Test Messages**
```bash
export PUBSUB_EMULATOR_HOST=localhost:8085
npm run pubsub:publish normal
```

## Test Script Commands

```bash
npm run pubsub:setup                    # Create topics/subscriptions
npm run pubsub:publish normal           # Normal test case
npm run pubsub:publish boundary         # Boundary values test
npm run pubsub:publish invalid_json     # Invalid JSON test
npm run pubsub:publish missing_fields   # Missing fields test
npm run pubsub:listen                   # Listen (Ctrl+C to stop)
npm run pubsub:listen 60                # Listen (60s timeout)
```

## Test Scenarios

| Scenario | Description |
|----------|-------------|
| `normal` | Standard case: 0.75 recaptcha, SMS & bio verified |
| `boundary` | Edge case: perfect recaptcha (1.0), all verified |
| `invalid_json` | Malformed JSON for error handling test |
| `missing_fields` | Missing `bio_verified` field |

## Environment Variables

Set via `.env` or manually:

```bash
PUBSUB_EMULATOR_HOST=localhost:8085
GCP_PROJECT_ID=test-project
PROVER_TOPIC=prover-requests
PROVER_SUBSCRIPTION=prover-requests-sub
RESULT_TOPIC=prover-results
RESULT_SUBSCRIPTION=prover-results-sub
MAX_CONCURRENT_PROOFS=2
PROOF_TIMEOUT_SECS=300
```

Load from file:
```bash
export $(grep -v '^#' .env | xargs)
```

## Expected Results

### Successful Proof

```json
{
  "request_id": "test-normal-001",
  "status": "success",
  "proof_data": {
    "proof": "0x...",
    "public_values": "0x..."
  },
  "metrics": {
    "proof_generation_secs": 45.2,
    "total_processing_secs": 45.5
  },
  "timestamp": "2024-01-01T12:00:00Z"
}
```

### Failed Proof

```json
{
  "request_id": "test-invalid-001",
  "status": "failed",
  "error": {
    "code": "PROOF_GENERATION_ERROR",
    "message": "Failed to generate proof: invalid input"
  },
  "timestamp": "2024-01-01T12:00:00Z"
}
```
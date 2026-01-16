# Prover Service Testing Guide

This guide covers how to test the ZK prover service locally using the Google Cloud Pub/Sub Emulator.

## Prerequisites

- Docker (for Pub/Sub Emulator)
- Python 3.7+
- Python packages: `google-cloud-pubsub`

## Setup

### 1. Install Python Dependencies

```bash
./scripts/setup-test-env.sh
```

This creates a virtual environment and installs required packages.

### 2. Start the Pub/Sub Emulator

```bash
./scripts/start-emulator.sh
```

This starts the emulator via Docker on `localhost:8085`.

To stop the emulator:
```bash
./scripts/stop-emulator.sh
```

## Running Tests

### Manual Testing (4 Terminals)

**Terminal 1: Start Emulator**
```bash
./scripts/start-emulator.sh
```

**Terminal 2: Setup and Listen for Results**
```bash
export PUBSUB_EMULATOR_HOST=localhost:8085
python scripts/test-pubsub.py setup
python scripts/test-pubsub.py listen forever
```

**Terminal 3: Run Prover Service**
```bash
cargo run --release --bin prover
```

**Terminal 4: Send Test Messages**
```bash
export PUBSUB_EMULATOR_HOST=localhost:8085
python scripts/test-pubsub.py publish normal
```

## Test Script Commands

Use `test-pubsub.py` directly (requires environment variables):

```bash
python scripts/test-pubsub.py setup              # Create topics/subscriptions
python scripts/test-pubsub.py publish normal     # Normal test case
python scripts/test-pubsub.py publish boundary   # Boundary values test
python scripts/test-pubsub.py publish invalid_json    # Invalid JSON test
python scripts/test-pubsub.py publish missing_fields  # Missing fields test
python scripts/test-pubsub.py listen              # Listen (30s timeout)
python scripts/test-pubsub.py listen forever      # Listen forever (Ctrl+C to stop)
python scripts/test-pubsub.py e2e                 # Full E2E test
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
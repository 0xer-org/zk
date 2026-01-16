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

### Quick E2E Test

The simplest way to test:

```bash
# Terminal 1: Start emulator
./scripts/start-emulator.sh

# Terminal 2: Run E2E test
./scripts/run-test.sh e2e
```

### Manual Testing (4 Terminals)

For more control over the testing workflow:

**Terminal 1: Start Emulator**
```bash
./scripts/start-emulator.sh
```

**Terminal 2: Setup and Listen for Results**
```bash
export $(cat .env.test | xargs)
python scripts/test-pubsub.py setup
python scripts/test-pubsub.py listen 120
```

**Terminal 3: Run Prover Service**
```bash
export $(cat .env.test | xargs)
cd prover && cargo run --release
```

**Terminal 4: Send Test Messages**
```bash
export $(cat .env.test | xargs)
python scripts/test-pubsub.py publish normal
```

## Test Script Commands

The `run-test.sh` wrapper handles environment setup automatically:

```bash
./scripts/run-test.sh e2e                # Complete E2E test (listens forever, Ctrl+C to stop)
./scripts/run-test.sh publish normal     # Publish normal test message
./scripts/run-test.sh publish boundary   # Publish boundary test message
./scripts/run-test.sh listen 60          # Listen for 60 seconds
```

Or use `test-pubsub.py` directly (requires environment variables):

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

Set via `.env.test` or manually:

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
export $(cat .env.test | xargs)
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

## Troubleshooting

### Emulator Not Starting
```bash
# Check if port is in use
lsof -i :8085

# Kill existing process
kill $(lsof -t -i:8085)
```

### Connection Refused
- Ensure `PUBSUB_EMULATOR_HOST` is set
- Verify emulator is running: `docker ps | grep pubsub`

### No Messages Received
- Run setup again: `python scripts/test-pubsub.py setup`
- Check prover service logs
- Emulator stores data in memory; restart requires re-running setup

## Files Overview

```
scripts/
├── start-emulator.sh      # Start Pub/Sub Emulator (Docker)
├── stop-emulator.sh       # Stop Pub/Sub Emulator
├── setup-test-env.sh      # Install Python dependencies
├── run-test.sh            # Test wrapper with env setup
├── test-pubsub.py         # Main test script
├── requirements.txt       # Python dependencies
└── README.md              # Quick reference

.env.test                  # Test environment config
```

## Next Steps

After local testing succeeds:
1. Review [pubsub-setup-guide.md](./pubsub-setup-guide.md) for production setup
2. Configure GCP Pub/Sub in your project
3. Set up proper service account permissions
4. Deploy the prover service

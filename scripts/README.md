# Test Scripts Guide

Quick reference for testing the ZK Prover Pub/Sub service locally.

## Prerequisites

### Docker (for Pub/Sub Emulator)

The Pub/Sub emulator runs in Docker, so make sure Docker is installed and running:

```bash
# Check Docker is running
docker --version
```

### Python Dependencies

Install the required Python packages for testing:

```bash
./scripts/setup-test-env.sh
```

## Quick Start

### Option 1: Automated E2E Test

```bash
# Terminal 1: Start emulator
./scripts/start-emulator.sh

# Terminal 2: Run end-to-end test
./scripts/run-test.sh e2e
```

### Option 2: Manual Testing

```bash
# Terminal 1: Start emulator
./scripts/start-emulator.sh

# Terminal 2: Setup and listen
export $(cat .env.test | xargs)
python scripts/test-pubsub.py setup
python scripts/test-pubsub.py listen 120

# Terminal 3: Run prover service
export $(cat .env.test | xargs)
cd prover && cargo run --release

# Terminal 4: Send test messages
export $(cat .env.test | xargs)
python scripts/test-pubsub.py publish normal
```

## Available Scripts

### `setup-test-env.sh`
Installs Python dependencies required for testing.

```bash
./scripts/setup-test-env.sh
```

### `start-emulator.sh`
Starts the Google Cloud Pub/Sub Emulator on localhost:8085 using Docker.

```bash
./scripts/start-emulator.sh
```

### `test-pubsub.py`
Main test script with multiple commands.

**Setup topics and subscriptions:**
```bash
python scripts/test-pubsub.py setup
```

**Publish test messages:**
```bash
python scripts/test-pubsub.py publish normal       # Normal case
python scripts/test-pubsub.py publish boundary     # Boundary values
python scripts/test-pubsub.py publish invalid_json # Invalid JSON
python scripts/test-pubsub.py publish missing_fields # Missing fields
```

**Listen for results:**
```bash
python scripts/test-pubsub.py listen              # 30 second timeout
python scripts/test-pubsub.py listen 60           # 60 second timeout
```

**Run E2E test:**
```bash
python scripts/test-pubsub.py e2e
```

### `run-test.sh`
Wrapper script that sets up environment and runs tests.

```bash
./scripts/run-test.sh e2e                # Run E2E test (listens forever, Ctrl+C to stop)
./scripts/run-test.sh publish [scenario] # Publish test message
./scripts/run-test.sh listen [timeout]   # Listen for results
```

## Test Scenarios

Built-in test scenarios in `test-pubsub.py`:

| Scenario | Description |
|----------|-------------|
| `normal` | Standard case: 0.75 recaptcha, SMS & bio verified |
| `boundary` | Edge case: perfect recaptcha (1.0), all verified |
| `invalid_json` | Malformed JSON for error handling test |
| `missing_fields` | Missing `bio_verified` field |

## Environment Variables

Set these before running tests (or use `.env.test`):

```bash
export PUBSUB_EMULATOR_HOST=localhost:8085
export GCP_PROJECT_ID=test-project
export PROVER_TOPIC=prover-requests
export PROVER_SUBSCRIPTION=prover-requests-sub
export RESULT_TOPIC=prover-results
export RESULT_SUBSCRIPTION=prover-results-sub
```

Or load from `.env.test`:
```bash
export $(cat .env.test | xargs)
```

## Troubleshooting

### Emulator not starting
```bash
# Check if port is in use
lsof -i :8085

# Kill existing process
kill $(lsof -t -i:8085)
```

### Connection refused
- Ensure `PUBSUB_EMULATOR_HOST` is set
- Verify emulator is running: `ps aux | grep pubsub`

### No messages received
- Run setup again: `python scripts/test-pubsub.py setup`
- Check prover service logs
- Verify emulator is running

## More Information

See [testing-guide.md](../Doc/testing-guide.md) for comprehensive documentation.

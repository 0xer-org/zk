#!/bin/bash
# Complete test workflow helper script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
VENV_DIR="$PROJECT_DIR/.venv"

# Activate virtual environment if it exists
if [ -d "$VENV_DIR" ]; then
    source "$VENV_DIR/bin/activate"
else
    echo "⚠️  Virtual environment not found. Run ./scripts/setup-test-env.sh first."
    exit 1
fi

echo "🧪 ZK Prover Service Test Suite"
echo "================================"
echo ""

# Check if emulator is running
if ! nc -z localhost 8085 2>/dev/null; then
    echo "❌ Pub/Sub Emulator is not running on localhost:8085"
    echo "   Please start it first:"
    echo "   ./scripts/start-emulator.sh"
    exit 1
fi

echo "✓ Pub/Sub Emulator is running"

# Set environment variables
export PUBSUB_EMULATOR_HOST=localhost:8085
export GCP_PROJECT_ID=test-project
export PROVER_TOPIC=prover-requests
export PROVER_SUBSCRIPTION=prover-requests-sub
export RESULT_TOPIC=prover-results
export RESULT_SUBSCRIPTION=prover-results-sub

echo "✓ Environment configured"
echo ""

# Setup topics and subscriptions
echo "📋 Setting up topics and subscriptions..."
python3 scripts/test-pubsub.py setup
echo ""

# Run tests based on argument
if [ "$1" == "e2e" ]; then
    echo "🔄 Running end-to-end test..."
    python3 scripts/test-pubsub.py e2e
elif [ "$1" == "publish" ]; then
    SCENARIO=${2:-normal}
    echo "📤 Publishing test message (scenario: $SCENARIO)..."
    python3 scripts/test-pubsub.py publish "$SCENARIO"
elif [ "$1" == "listen" ]; then
    TIMEOUT=${2:-30}
    echo "👂 Listening for results (${TIMEOUT}s timeout)..."
    python3 scripts/test-pubsub.py listen "$TIMEOUT"
else
    echo "Usage:"
    echo "  ./scripts/run-test.sh e2e              # Run complete E2E test"
    echo "  ./scripts/run-test.sh publish [type]   # Publish test message"
    echo "  ./scripts/run-test.sh listen [timeout] # Listen for results"
    echo ""
    echo "Publish types: normal, boundary, invalid_json, missing_fields"
    exit 1
fi

echo ""
echo "✓ Test complete!"

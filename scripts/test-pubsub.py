#!/usr/bin/env python3
"""
Test script for sending messages to Pub/Sub emulator.
This script creates test topics/subscriptions and sends test messages to the prover service.
"""

import json
import base64
import os
import sys
from google.cloud import pubsub_v1
import time

# Configuration
PROJECT_ID = os.getenv("GCP_PROJECT_ID", "test-project")
PROVER_TOPIC = os.getenv("PROVER_TOPIC", "prover-requests")
PROVER_SUBSCRIPTION = os.getenv("PROVER_SUBSCRIPTION", "prover-requests-sub")
RESULT_TOPIC = os.getenv("RESULT_TOPIC", "prover-results")
RESULT_SUBSCRIPTION = os.getenv("RESULT_SUBSCRIPTION", "prover-results-sub")

# For emulator, set this environment variable
# PUBSUB_EMULATOR_HOST=localhost:8085

def create_topics_and_subscriptions():
    """Create necessary topics and subscriptions for testing."""
    publisher = pubsub_v1.PublisherClient()
    subscriber = pubsub_v1.SubscriberClient()

    # Create topics
    prover_topic_path = publisher.topic_path(PROJECT_ID, PROVER_TOPIC)
    result_topic_path = publisher.topic_path(PROJECT_ID, RESULT_TOPIC)

    try:
        publisher.create_topic(request={"name": prover_topic_path})
        print(f"✓ Created topic: {prover_topic_path}")
    except Exception as e:
        print(f"Topic {PROVER_TOPIC} may already exist: {e}")

    try:
        publisher.create_topic(request={"name": result_topic_path})
        print(f"✓ Created topic: {result_topic_path}")
    except Exception as e:
        print(f"Topic {RESULT_TOPIC} may already exist: {e}")

    # Create subscriptions
    prover_sub_path = subscriber.subscription_path(PROJECT_ID, PROVER_SUBSCRIPTION)
    result_sub_path = subscriber.subscription_path(PROJECT_ID, RESULT_SUBSCRIPTION)

    try:
        subscriber.create_subscription(
            request={"name": prover_sub_path, "topic": prover_topic_path}
        )
        print(f"✓ Created subscription: {prover_sub_path}")
    except Exception as e:
        print(f"Subscription {PROVER_SUBSCRIPTION} may already exist: {e}")

    try:
        subscriber.create_subscription(
            request={"name": result_sub_path, "topic": result_topic_path}
        )
        print(f"✓ Created subscription: {result_sub_path}")
    except Exception as e:
        print(f"Subscription {RESULT_SUBSCRIPTION} may already exist: {e}")

def create_test_message(request_id: str, scenario: str = "normal"):
    """
    Create test message based on scenario.

    Message format matches ProverRequest:
    - verification_results (private): recaptcha_score, sms_verified, bio_verified
    - public_inputs: w1, w2, w3, w4, expected_output

    All decimal values use fixed-point scale of 10,000:
    - 0.75 -> 7500
    - 1.0 -> 10000

    Formula: floor((W1 + W2 * recaptchaScore + W3 * smsVerified + W4 * bioVerified) * 255 / SCALE)
    """

    if scenario == "normal":
        # Standard verification: 0.75 recaptcha, SMS verified, bio verified
        # Expected: floor((1500 + 2000*0.75 + 2500*1 + 4000*1) * 255 / 10000) = 204
        message = {
            "request_id": request_id,
            "verification_results": {
                "recaptcha_score": 7500,  # 0.75 in fixed-point
                "sms_verified": 1,
                "bio_verified": 1
            },
            "public_inputs": {
                "w1": 1500,    # 0.15
                "w2": 2000,    # 0.2
                "w3": 2500,    # 0.25
                "w4": 4000,    # 0.4
                "expected_output": 204
            }
        }
    elif scenario == "boundary":
        # Maximum values: perfect recaptcha (1.0), all verified
        # Expected: floor((1500 + 2000*1.0 + 2500*1 + 4000*1) * 255 / 10000) = 255
        message = {
            "request_id": request_id,
            "verification_results": {
                "recaptcha_score": 10000,  # 1.0 in fixed-point (max)
                "sms_verified": 1,
                "bio_verified": 1
            },
            "public_inputs": {
                "w1": 1500,
                "w2": 2000,
                "w3": 2500,
                "w4": 4000,
                "expected_output": 255
            }
        }
    elif scenario == "invalid_json":
        return "{ invalid json }"
    elif scenario == "missing_fields":
        # Missing bio_verified field to test error handling
        message = {
            "request_id": request_id,
            "verification_results": {
                "recaptcha_score": 7500,
                "sms_verified": 1
                # missing bio_verified - will cause deserialization error
            },
            "public_inputs": {
                "w1": 1500,
                "w2": 2000,
                "w3": 2500,
                "w4": 4000,
                "expected_output": 204
            }
        }
    else:
        raise ValueError(f"Unknown scenario: {scenario}")

    return json.dumps(message)

def publish_test_message(scenario: str = "normal", request_id: str = None):
    """Publish a test message to the prover topic."""
    publisher = pubsub_v1.PublisherClient()
    topic_path = publisher.topic_path(PROJECT_ID, PROVER_TOPIC)

    if request_id is None:
        request_id = f"test-{scenario}-{int(time.time())}"

    message_data = create_test_message(request_id, scenario)
    data = message_data.encode("utf-8")

    future = publisher.publish(topic_path, data)
    message_id = future.result()

    print(f"✓ Published message: {message_id}")
    print(f"  Request ID: {request_id}")
    print(f"  Scenario: {scenario}")
    print(f"  Data: {message_data[:100]}..." if len(message_data) > 100 else f"  Data: {message_data}")

    return message_id

def listen_for_results(timeout: int = 30):
    """Listen for result messages."""
    subscriber = pubsub_v1.SubscriberClient()
    subscription_path = subscriber.subscription_path(PROJECT_ID, RESULT_SUBSCRIPTION)

    print(f"\n📡 Listening for results on {subscription_path}...")
    print(f"   Timeout: {timeout} seconds")

    results = []

    def callback(message):
        print(f"\n✓ Received result message:")
        try:
            data = json.loads(message.data.decode("utf-8"))
            print(json.dumps(data, indent=2))
            results.append(data)
        except Exception as e:
            print(f"  Error parsing message: {e}")
            print(f"  Raw data: {message.data}")
        message.ack()

    streaming_pull_future = subscriber.subscribe(subscription_path, callback=callback)

    try:
        if timeout is None:
            print("   Mode: forever (Ctrl+C to stop)")
            streaming_pull_future.result()
        else:
            streaming_pull_future.result(timeout=timeout)
    except KeyboardInterrupt:
        print("\n\n⛔ Stopped by user")
        streaming_pull_future.cancel()
    except Exception as e:
        print(f"\n⏱ Timeout or error: {e}")
        streaming_pull_future.cancel()

    return results

def main():
    """Main test function."""
    if len(sys.argv) < 2:
        print("Usage:")
        print("  python scripts/test-pubsub.py setup          # Create topics and subscriptions")
        print("  python scripts/test-pubsub.py publish <scenario>  # Publish test message")
        print("  python scripts/test-pubsub.py listen         # Listen for results")
        print("  python scripts/test-pubsub.py e2e            # End-to-end test")
        print("\nScenarios: normal, boundary, invalid_json, missing_fields")
        print("\nEnvironment variables:")
        print(f"  PUBSUB_EMULATOR_HOST: {os.getenv('PUBSUB_EMULATOR_HOST', 'not set')}")
        print(f"  GCP_PROJECT_ID: {PROJECT_ID}")
        print(f"  PROVER_TOPIC: {PROVER_TOPIC}")
        print(f"  RESULT_TOPIC: {RESULT_TOPIC}")
        sys.exit(1)

    command = sys.argv[1]

    if command == "setup":
        print("🔧 Setting up topics and subscriptions...")
        create_topics_and_subscriptions()
        print("\n✓ Setup complete!")

    elif command == "publish":
        scenario = sys.argv[2] if len(sys.argv) > 2 else "normal"
        print(f"📤 Publishing test message (scenario: {scenario})...")
        publish_test_message(scenario)
        print("\n✓ Message published!")

    elif command == "listen":
        timeout_arg = sys.argv[2] if len(sys.argv) > 2 else "30"
        if timeout_arg == "forever":
            results = listen_for_results(timeout=None)
        else:
            results = listen_for_results(timeout=int(timeout_arg))
        print(f"\n✓ Received {len(results)} result(s)")

    elif command == "e2e":
        print("🧪 Running end-to-end test...")
        print("\n1️⃣ Setting up topics and subscriptions...")
        create_topics_and_subscriptions()

        print("\n2️⃣ Publishing test message...")
        request_id = f"e2e-test-{int(time.time())}"
        publish_test_message("normal", request_id)

        print("\n3️⃣ Waiting for results...")
        results = listen_for_results(timeout=None)

        if results:
            print(f"\n✓ E2E test complete! Received {len(results)} result(s)")
            for i, result in enumerate(results, 1):
                print(f"\nResult {i}:")
                print(json.dumps(result, indent=2))
        else:
            print("\n⚠ No results received within timeout period")

    else:
        print(f"Unknown command: {command}")
        sys.exit(1)

if __name__ == "__main__":
    main()

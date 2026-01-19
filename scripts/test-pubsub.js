#!/usr/bin/env node
/**
 * Test script for sending messages to Pub/Sub emulator.
 * This script creates test topics/subscriptions and sends test messages to the prover service.
 */

import { PubSub } from "@google-cloud/pubsub";

// Configuration
const PROJECT_ID = process.env.GCP_PROJECT_ID || "test-project";
const PROVER_TOPIC = process.env.PROVER_TOPIC || "prover-requests";
const PROVER_SUBSCRIPTION =
  process.env.PROVER_SUBSCRIPTION || "prover-requests-sub";
const RESULT_TOPIC = process.env.RESULT_TOPIC || "prover-results";
const RESULT_SUBSCRIPTION =
  process.env.RESULT_SUBSCRIPTION || "prover-results-sub";

// For emulator, set this environment variable
// PUBSUB_EMULATOR_HOST=localhost:8085

const pubsub = new PubSub({ projectId: PROJECT_ID });

async function createTopicsAndSubscriptions() {
  // Create topics
  try {
    await pubsub.createTopic(PROVER_TOPIC);
    console.log(`✓ Created topic: ${PROVER_TOPIC}`);
  } catch (e) {
    console.log(`Topic ${PROVER_TOPIC} may already exist: ${e.message}`);
  }

  try {
    await pubsub.createTopic(RESULT_TOPIC);
    console.log(`✓ Created topic: ${RESULT_TOPIC}`);
  } catch (e) {
    console.log(`Topic ${RESULT_TOPIC} may already exist: ${e.message}`);
  }

  // Create subscriptions
  try {
    await pubsub.topic(PROVER_TOPIC).createSubscription(PROVER_SUBSCRIPTION, {
      ackDeadlineSeconds: 600, // Max allowed is 600 seconds (10 min)
    });
    console.log(
      `✓ Created subscription: ${PROVER_SUBSCRIPTION} (ack_deadline=600s)`
    );
  } catch (e) {
    console.log(
      `Subscription ${PROVER_SUBSCRIPTION} may already exist: ${e.message}`
    );
  }

  try {
    await pubsub.topic(RESULT_TOPIC).createSubscription(RESULT_SUBSCRIPTION);
    console.log(`✓ Created subscription: ${RESULT_SUBSCRIPTION}`);
  } catch (e) {
    console.log(
      `Subscription ${RESULT_SUBSCRIPTION} may already exist: ${e.message}`
    );
  }
}

function createTestMessage(requestId, scenario = "normal") {
  /**
   * Create test message based on scenario.
   *
   * Message format matches ProverRequest:
   * - verification_results (private): recaptcha_score, sms_verified, bio_verified
   * - public_inputs: w1, w2, w3, w4, expected_output
   *
   * All decimal values use fixed-point scale of 10,000:
   * - 0.75 -> 7500
   * - 1.0 -> 10000
   *
   * Formula: floor((W1 + W2 * recaptchaScore + W3 * smsVerified + W4 * bioVerified) * 255 / SCALE)
   */

  let message;

  if (scenario === "normal") {
    // Standard verification: 0.75 recaptcha, SMS verified, bio verified
    // Expected: floor((1500 + 2000*0.75 + 2500*1 + 4000*1) * 255 / 10000) = 204
    message = {
      request_id: requestId,
      verification_results: {
        recaptcha_score: 7500, // 0.75 in fixed-point
        sms_verified: 1,
        bio_verified: 1,
      },
      public_inputs: {
        w1: 1500, // 0.15
        w2: 2000, // 0.2
        w3: 2500, // 0.25
        w4: 4000, // 0.4
        expected_output: 204,
      },
    };
  } else if (scenario === "boundary") {
    // Maximum values: perfect recaptcha (1.0), all verified
    // Expected: floor((1500 + 2000*1.0 + 2500*1 + 4000*1) * 255 / 10000) = 255
    message = {
      request_id: requestId,
      verification_results: {
        recaptcha_score: 10000, // 1.0 in fixed-point (max)
        sms_verified: 1,
        bio_verified: 1,
      },
      public_inputs: {
        w1: 1500,
        w2: 2000,
        w3: 2500,
        w4: 4000,
        expected_output: 255,
      },
    };
  } else if (scenario === "invalid_json") {
    return "{ invalid json }";
  } else if (scenario === "missing_fields") {
    // Missing bio_verified field to test error handling
    message = {
      request_id: requestId,
      verification_results: {
        recaptcha_score: 7500,
        sms_verified: 1,
        // missing bio_verified - will cause deserialization error
      },
      public_inputs: {
        w1: 1500,
        w2: 2000,
        w3: 2500,
        w4: 4000,
        expected_output: 204,
      },
    };
  } else {
    throw new Error(`Unknown scenario: ${scenario}`);
  }

  return JSON.stringify(message);
}

async function publishTestMessage(scenario = "normal", requestId = null) {
  const topic = pubsub.topic(PROVER_TOPIC);

  if (!requestId) {
    requestId = `test-${scenario}-${Math.floor(Date.now() / 1000)}`;
  }

  const messageData = createTestMessage(requestId, scenario);
  const data = Buffer.from(messageData);

  const messageId = await topic.publishMessage({ data });

  console.log(`✓ Published message: ${messageId}`);
  console.log(`  Request ID: ${requestId}`);
  console.log(`  Scenario: ${scenario}`);
  console.log(
    `  Data: ${messageData.length > 100 ? messageData.slice(0, 100) + "..." : messageData}`
  );

  return messageId;
}

async function listenForResults(timeout = 30) {
  const subscription = pubsub.subscription(RESULT_SUBSCRIPTION);

  console.log(`\n📡 Listening for results on ${RESULT_SUBSCRIPTION}...`);
  console.log(`   Timeout: ${timeout === null ? "forever" : timeout + " seconds"}`);

  const results = [];

  const messageHandler = (message) => {
    console.log(`\n✓ Received result message:`);
    try {
      const data = JSON.parse(message.data.toString());
      console.log(JSON.stringify(data, null, 2));
      results.push(data);
    } catch (e) {
      console.log(`  Error parsing message: ${e.message}`);
      console.log(`  Raw data: ${message.data}`);
    }
    message.ack();
  };

  subscription.on("message", messageHandler);

  return new Promise((resolve) => {
    if (timeout === null) {
      console.log("   Mode: forever (Ctrl+C to stop)");
      process.on("SIGINT", () => {
        console.log("\n\n⛔ Stopped by user");
        subscription.removeListener("message", messageHandler);
        resolve(results);
      });
    } else {
      setTimeout(() => {
        console.log(`\n⏱ Timeout reached`);
        subscription.removeListener("message", messageHandler);
        resolve(results);
      }, timeout * 1000);
    }
  });
}

async function main() {
  const args = process.argv.slice(2);

  if (args.length < 1) {
    console.log("Usage:");
    console.log(
      "  node scripts/test-pubsub.js setup          # Create topics and subscriptions"
    );
    console.log(
      "  node scripts/test-pubsub.js publish <scenario>  # Publish test message"
    );
    console.log(
      "  node scripts/test-pubsub.js listen         # Listen for results"
    );
    console.log("\nScenarios: normal, boundary, invalid_json, missing_fields");
    console.log("\nEnvironment variables:");
    console.log(
      `  PUBSUB_EMULATOR_HOST: ${process.env.PUBSUB_EMULATOR_HOST || "not set"}`
    );
    console.log(`  GCP_PROJECT_ID: ${PROJECT_ID}`);
    console.log(`  PROVER_TOPIC: ${PROVER_TOPIC}`);
    console.log(`  RESULT_TOPIC: ${RESULT_TOPIC}`);
    process.exit(1);
  }

  const command = args[0];

  if (command === "setup") {
    console.log("🔧 Setting up topics and subscriptions...");
    await createTopicsAndSubscriptions();
    console.log("\n✓ Setup complete!");
  } else if (command === "publish") {
    const scenario = args[1] || "normal";
    console.log(`📤 Publishing test message (scenario: ${scenario})...`);
    await publishTestMessage(scenario);
    console.log("\n✓ Message published!");
  } else if (command === "listen") {
    const timeoutArg = args[1];
    let results;
    if (!timeoutArg || timeoutArg === "forever") {
      results = await listenForResults(null);
    } else {
      results = await listenForResults(parseInt(timeoutArg));
    }
    console.log(`\n✓ Received ${results.length} result(s)`);
  } else {
    console.log(`Unknown command: ${command}`);
    process.exit(1);
  }
}

main().catch(console.error);

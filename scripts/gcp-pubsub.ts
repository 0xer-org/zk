#!/usr/bin/env tsx
/**
 * GCP Pub/Sub script for interacting with real GCP Pub/Sub.
 * Uses @google-cloud/pubsub SDK for proper authentication.
 */

import { PubSub, Message } from "@google-cloud/pubsub";
import { writeFileSync, mkdirSync } from "fs";

// Configuration
const PROJECT_ID = process.env.GCP_PROJECT_ID || "test-project";
const PROVER_TOPIC = process.env.PROVER_TOPIC || "prover-requests";
const RESULT_SUBSCRIPTION =
  process.env.RESULT_SUBSCRIPTION || "prover-results-sub";

const pubsub = new PubSub({ projectId: PROJECT_ID });

type Scenario = "normal" | "boundary" | "invalid_json" | "missing_fields";

interface VerificationResults {
  recaptcha_score: number;
  sms_verified: number;
  bio_verified?: number;
}

interface PublicInputs {
  w1: number;
  w2: number;
  w3: number;
  w4: number;
  expected_output: number;
}

interface ProverRequest {
  request_id: string;
  verification_results: VerificationResults;
  public_inputs: PublicInputs;
}

interface ProofData {
  proof: string;
  public_inputs: string;
  verification_key: string;
  human_index: number;
}

interface ProverResult {
  request_id: string;
  status: string;
  proof_data?: ProofData;
  error?: unknown;
  metrics?: unknown;
}

const PROOFS_DIR = "prover/data/proofs";

function saveProofAsInputs(proofData: ProofData, requestId: string): void {
  const proof = JSON.parse(Buffer.from(proofData.proof, "base64").toString());
  const publicValues = Buffer.from(proofData.public_inputs, "base64").toString();
  const riscvVKey = Buffer.from(proofData.verification_key, "base64").toString();

  const inputs = { proof, publicValues, riscvVKey };

  mkdirSync(PROOFS_DIR, { recursive: true });
  const proofPath = `${PROOFS_DIR}/${requestId}.json`;
  writeFileSync(proofPath, JSON.stringify(inputs, null, 2));
  console.log(`\n💾 Saved proof to ${proofPath}`);
  console.log(`   Run 'npm run verify ${proofPath}' to verify on-chain`);
}

function createTestMessage(requestId: string, scenario: Scenario): string {
  let message: ProverRequest;

  if (scenario === "normal") {
    message = {
      request_id: requestId,
      verification_results: {
        recaptcha_score: 7500,
        sms_verified: 1,
        bio_verified: 1,
      },
      public_inputs: {
        w1: 1500,
        w2: 2000,
        w3: 2500,
        w4: 4000,
        expected_output: 204,
      },
    };
  } else if (scenario === "boundary") {
    message = {
      request_id: requestId,
      verification_results: {
        recaptcha_score: 10000,
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
    message = {
      request_id: requestId,
      verification_results: {
        recaptcha_score: 7500,
        sms_verified: 1,
      },
      public_inputs: {
        w1: 1500,
        w2: 2000,
        w3: 2500,
        w4: 4000,
        expected_output: 204,
      },
    } as ProverRequest;
  } else {
    throw new Error(`Unknown scenario: ${scenario}`);
  }

  return JSON.stringify(message);
}

async function publishTestMessage(
  scenario: Scenario = "normal",
  requestId?: string
): Promise<string> {
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

async function listenForResults(
  timeout: number | null = null
): Promise<ProverResult[]> {
  const subscription = pubsub.subscription(RESULT_SUBSCRIPTION);

  console.log(`\n📡 Listening for results on ${RESULT_SUBSCRIPTION}...`);
  console.log(`   Project: ${PROJECT_ID}`);
  console.log(
    `   Timeout: ${timeout === null ? "forever (Ctrl+C to stop)" : timeout + " seconds"}`
  );

  const results: ProverResult[] = [];

  return new Promise((resolve) => {
    let timeoutId: ReturnType<typeof setTimeout> | null = null;

    const cleanup = () => {
      subscription.removeListener("message", messageHandler);
      subscription.removeListener("error", errorHandler);
      if (timeoutId) clearTimeout(timeoutId);
    };

    const messageHandler = (message: Message) => {
      console.log(`\n✓ Received result message:`);
      try {
        const decoded = message.data.toString();
        const data = JSON.parse(decoded) as ProverResult;
        console.log(JSON.stringify(data, null, 2));
        results.push(data);

        if (data.status === "success" && data.proof_data) {
          saveProofAsInputs(data.proof_data, data.request_id);
        }
      } catch (e) {
        const err = e as Error;
        console.log(`  Error parsing message: ${err.message}`);
        console.log(`  Raw data: ${message.data.toString()}`);
      }
      message.ack();
    };

    const errorHandler = (error: Error) => {
      console.error(`\n❌ Subscription error: ${error.message}`);
    };

    subscription.on("message", messageHandler);
    subscription.on("error", errorHandler);

    process.once("SIGINT", () => {
      console.log("\n\n⛔ Stopped by user");
      cleanup();
      resolve(results);
      setTimeout(() => process.exit(0), 100);
    });

    if (timeout !== null) {
      timeoutId = setTimeout(() => {
        console.log(`\n⏱ Timeout reached`);
        cleanup();
        resolve(results);
      }, timeout * 1000);
    }
  });
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);

  if (args.length < 1) {
    console.log("GCP Pub/Sub Script - For real GCP Pub/Sub interaction\n");
    console.log("Usage:");
    console.log("  npm run gcp:publish <scenario>  # Publish test message");
    console.log("  npm run gcp:listen [timeout]    # Listen for results\n");
    console.log("Scenarios: normal, boundary, invalid_json, missing_fields\n");
    console.log("Environment:");
    console.log(`  GCP_PROJECT_ID: ${PROJECT_ID}`);
    console.log(`  PROVER_TOPIC: ${PROVER_TOPIC}`);
    console.log(`  RESULT_SUBSCRIPTION: ${RESULT_SUBSCRIPTION}`);
    process.exit(1);
  }

  const command = args[0];

  if (command === "publish") {
    const scenario = (args[1] || "normal") as Scenario;
    console.log(`📤 Publishing test message (scenario: ${scenario})...`);
    await publishTestMessage(scenario);
    console.log("\n✓ Message published!");
  } else if (command === "listen") {
    const timeoutArg = args[1];
    let results: ProverResult[];
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
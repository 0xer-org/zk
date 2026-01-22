# Human Index ZKP with Pico

A complete zero-knowledge proof system for calculating and verifying a human verification index, from proof generation to on-chain verification using the Pico zkVM framework.

## Overview

This project implements a full ZKP pipeline that:

1. **Compiles the ZKP circuit** (one-time setup)
2. **Generates the verifier contract** directly from the circuit (one-time setup)
3. **Deploys a Solidity verifier** contract to Ethereum and BSC networks (one-time setup)
4. **Runs a Pub/Sub prover service** that receives proof requests and returns results via Google Cloud Pub/Sub
5. **Generates proofs** of correct human index calculation without revealing private verification data (repeatable)
6. **Verifies proofs on-chain** using the deployed contract (repeatable)

## Pub/Sub Prover Service

The prover can run as a cloud-native microservice using Google Cloud Pub/Sub:

```
┌─────────────────┐
│   Pub/Sub       │
│   Topic         │
│  (Requests)     │
└────────┬────────┘
         │
         │ Subscribe
         ▼
┌─────────────────────────────────────────┐
│        Prover Service                   │
│  ┌───────────────────────────────┐     │
│  │  Subscription Handler         │     │
│  │  - Receives messages          │     │
│  │  - Semaphore (2-4 concurrent) │     │
│  │  - ACK/NACK logic             │     │
│  └───────────┬───────────────────┘     │
│              │                          │
│              ▼                          │
│  ┌───────────────────────────────┐     │
│  │  Proof Generator              │     │
│  │  - Cached ELF                 │     │
│  │  - Blocking proof generation  │     │
│  │  - Base64 encoding            │     │
│  └───────────┬───────────────────┘     │
│              │                          │
│              ▼                          │
│  ┌───────────────────────────────┐     │
│  │  Result Publisher             │     │
│  │  - Publishes to result topic  │     │
│  │  - Includes metrics           │     │
│  └───────────────────────────────┘     │
└─────────────────────────────────────────┘
         │
         │ Publish
         ▼
┌─────────────────┐
│   Pub/Sub       │
│   Topic         │
│   (Results)     │
└─────────────────┘
```

### Formula

```bash
humanIndex = floor((W1 + W2 * recaptchaScore + W3 * smsVerified + W4 * bioVerified) * 255)
```

### Privacy Model

- **Private Inputs** (hidden in the proof):
  - `recaptchaScore`: Score from reCAPTCHA verification (0.0 to 1.0)
  - `smsVerified`: Whether SMS verification passed (0 or 1)
  - `bioVerified`: Whether biometric verification passed (0 or 1)

- **Public Inputs** (committed to the proof):
  - `W1`, `W2`, `W3`, `W4`: Weight parameters for the calculation
  - `expected_output`: The computed human index value

## References

- [Pico Documentation](https://pico-docs.brevis.network/)

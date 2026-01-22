# GCP Pub/Sub and Prover Service Deployment Guide

This guide will help you deploy the Pub/Sub architecture and Prover Service to Google Cloud Platform (GCP).

---

## Step 1: GCP Project and Pub/Sub Resource Setup

Before deploying the code, you need to set up the necessary infrastructure on GCP.

### 1. Prerequisites

Ensure you have installed the `gcloud` CLI and completed authentication:

```bash
gcloud auth login --no-launch-browser
gcloud config set project [YOUR_PROJECT_ID]
```

### 2. Enable Required APIs

```bash
gcloud services enable pubsub.googleapis.com
gcloud services enable compute.googleapis.com
gcloud services enable artifactregistry.googleapis.com
```

Wait a few minutes until the following response appears:

``` txt
Operation "operations/acf.p2-210193831987-0e03b4ff-f8ba-46cb-be76-f9adc4fed906" finished successfully.
```

### 3. Create Service Account (SA)

The Prover Service requires a dedicated identity to access Pub/Sub and Artifact Registry.

```bash
# Create Service Account
gcloud iam service-accounts create prover-service-sa \
    --display-name="Prover Service Account"

# Grant permissions
gcloud projects add-iam-policy-binding [YOUR_PROJECT_ID] \
    --member="serviceAccount:prover-service-sa@[YOUR_PROJECT_ID].iam.gserviceaccount.com" \
    --role="roles/pubsub.subscriber"

gcloud projects add-iam-policy-binding [YOUR_PROJECT_ID] \
    --member="serviceAccount:prover-service-sa@[YOUR_PROJECT_ID].iam.gserviceaccount.com" \
    --role="roles/pubsub.publisher"

gcloud projects add-iam-policy-binding [YOUR_PROJECT_ID] \
    --member="serviceAccount:prover-service-sa@[YOUR_PROJECT_ID].iam.gserviceaccount.com" \
    --role="roles/artifactregistry.reader"
```

### 4. Create Topics and Subscriptions

```bash
# Create Topics
gcloud pubsub topics create prover-requests
gcloud pubsub topics create prover-results

# Create Subscriptions
# Note: ack-deadline is set to 600s because proof generation takes a long time
gcloud pubsub subscriptions create prover-requests-sub \
    --topic=prover-requests \
    --ack-deadline=600

gcloud pubsub subscriptions create prover-results-sub \
    --topic=prover-results
```

**Verify resources have been created:**

```bash
gcloud pubsub topics list
gcloud pubsub subscriptions list
```

---

## Step 2: Prepare VM (Compute Engine)

We will create a Compute-optimized VM to run the Prover.

### 1. Choose Machine Specifications

* **Series**: C2 (Compute-optimized)
* **Recommended spec**: `c2-standard-8` (8 vCPU, 32GB RAM)
* **Disk**: At least 50GB SSD (PD-SSD)

### 2. Create VM

Use the following command to create a VM and attach the Service Account created earlier. Wait about 1-2 minutes for the VM to start.

* `--preemptible` (optional): Use Spot VM (cheaper, but may be interrupted, suitable for stateless Prover).

```bash
gcloud compute instances create prover-instance-1 \
    --zone=asia-east1-a \
    --machine-type=c2-standard-8 \
    --image-family=ubuntu-2204-lts \
    --image-project=ubuntu-os-cloud \
    --boot-disk-size=50GB \
    --boot-disk-type=pd-ssd \
    --service-account=prover-service-sa@[YOUR_PROJECT_ID].iam.gserviceaccount.com \
    --scopes=https://www.googleapis.com/auth/cloud-platform \
    --preemptible
```

---

## Step 3: Deploy Prover Service to VM

### 1. Build Docker Image Locally

#### A. Configure GCR Authentication

```bash
gcloud auth configure-docker
```

#### B. Build Image

Since the VM uses x86_64 architecture, macOS (ARM64) users need to specify the target platform:

```bash
docker build --platform linux/amd64 -t gcr.io/[YOUR_PROJECT_ID]/prover-service:latest .
```

This takes approximately 30-60 minutes to complete.

#### C. Push Image

```bash
docker push gcr.io/[YOUR_PROJECT_ID]/prover-service:latest
```

Depending on network speed, this takes approximately 1 hour.

### 2. Run on VM

SSH into your VM (first connection will generate SSH keys in local `~/.ssh/`):

```bash
gcloud compute ssh prover-instance-1
```

Execute the following inside the VM:

#### A. Install Docker

Install the latest version using the official Docker repository (to avoid Docker API version incompatibility issues):

```bash
sudo apt-get update
sudo apt-get install -y ca-certificates curl gnupg

# Add Docker official GPG key
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
sudo chmod a+r /etc/apt/keyrings/docker.gpg

# Add Docker repository
echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
  $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
  sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

# Install Docker
sudo apt-get update
sudo apt-get install -y docker-ce docker-ce-cli containerd.io

sudo usermod -aG docker $USER
# Log out and log back in for changes to take effect
exit
gcloud compute ssh prover-instance-1
```

#### B. Configure GCR Authentication

```bash
gcloud auth configure-docker
```

#### C. Prepare Host Directory (First Time or When Setup Files Updated)

The Prover Service internally calls another Docker container (`pico_gnark_cli`) to generate Groth16 proofs. Since we mount the Docker socket, the inner container is executed by the Host's Docker daemon, so we need to prepare shared directories on the Host:

```bash
# Create data directory on Host
sudo mkdir -p /app/data

# Pull the latest Image
docker pull gcr.io/[YOUR_PROJECT_ID]/prover-service:latest

# Copy Groth16 setup files from Image to Host
docker run --rm \
  -v /app/data:/host-data \
  gcr.io/[YOUR_PROJECT_ID]/prover-service:latest \
  cp /app/data/vm_pk /app/data/vm_vk /host-data/

# Verify files have been copied (vm_pk ~1.3GB, vm_vk ~520 bytes)
ls -la /app/data/
```

#### D. Set Up Application Default Credentials (ADC)

Sometimes the Container shows a `PermissionDenied` error after startup, indicating that the Rust Pub/Sub client cannot properly use GCE metadata authentication. To avoid this, you need to manually set up Application Default Credentials (ADC):

```bash
# Run on VM, this will give you a URL
gcloud auth application-default login --no-launch-browser
```

Open the URL in your local browser, log in to your Google account, then paste the authorization code back to the VM terminal.
This will create an ADC credentials file at `~/.config/gcloud/application_default_credentials.json` on the VM.

#### E. Start the Service

Use environment variables to set production parameters.

* `MAX_CONCURRENT_PROOFS`: Adjust based on your VM specifications (recommended 1 or 2 for c2-standard-8).

```bash
# Pull the latest Image (if updated)
docker pull gcr.io/[YOUR_PROJECT_ID]/prover-service:latest

# Manually stop and remove old Container (if exists)
docker stop prover-service && docker rm prover-service

docker run -d \
  --name prover-service \
  --restart always \
  --network host \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v /app/data:/app/data \
  -v ~/.config/gcloud/application_default_credentials.json:/app/credentials.json:ro \
  -e GOOGLE_APPLICATION_CREDENTIALS=/app/credentials.json \
  -e GCP_PROJECT_ID=[YOUR_PROJECT_ID] \
  -e PROVER_SUBSCRIPTION=prover-requests-sub \
  -e RESULT_TOPIC=prover-results \
  -e OUTPUT_DIR=/app/data \
  -e ELF_PATH=/app/app/elf/riscv32im-pico-zkvm-elf \
  -e MAX_CONCURRENT_PROOFS=2 \
  -e PROOF_TIMEOUT_SECS=3600 \
  -e RUST_LOG=info \
  gcr.io/[YOUR_PROJECT_ID]/prover-service:latest
```

**Note**:

* `-v /app/data:/app/data` mounts the Host data directory (required for Docker-in-Docker), ensuring Host and Container use the same path so Docker daemon can correctly find directories and files on the Host.
* `-v ~/.config/gcloud/application_default_credentials.json:/app/credentials.json:ro` mounts the ADC credentials file
* `-e GOOGLE_APPLICATION_CREDENTIALS=/app/credentials.json` tells the SDK where the credentials file is located

---

## Step 4: Verification and Monitoring

### 1. View Logs

Check if the Prover started correctly on the VM:

```bash
docker logs -f prover-service
```

You should see the message `Starting prover service, subscribing to 'prover-requests-sub'`.

### 2. Send Test Request

Run the test script on your local machine to send requests to the **real GCP Pub/Sub**.

Modify `.env` (or ensure environment variables are set correctly):

```bash
GCP_PROJECT_ID=[YOUR_PROJECT_ID]
# Ensure this line is commented out
# PUBSUB_EMULATOR_HOST=localhost:8085
```

Execute publish:

```bash
export $(grep -v '^#' .env | xargs)
npm run pubsub:publish normal
```

### 3. Verify Results

Go back to the VM log window, you should see it receiving the message and starting `Processing proof request`. Later, it will show `Proof generated successfully` and publish the result back to `prover-results`.

### 4. Receive Proof Results

The Prover Service publishes generated proofs to the `prover-results` Topic. Here are ways to receive the results:

#### A. Using gcloud CLI (for testing)

Pull messages once:

```bash
gcloud pubsub subscriptions pull prover-results-sub --auto-ack
```

Continuous monitoring (polling every second):

```bash
while true; do
  gcloud pubsub subscriptions pull prover-results-sub --auto-ack --format="json"
  sleep 1
done
```

> **Note**: `--auto-ack` automatically acknowledges messages after pulling, removing them from the queue. Without this parameter, messages will reappear after the ack deadline.

#### B. Using Built-in Project Script (Recommended)

The project includes a built-in TypeScript subscriber that can continuously listen for results:

```bash
export $(grep -v '^#' .env | xargs)
npm run pubsub:listen forever
```

After receiving a successful proof, the script will automatically save the result to `prover/data/groth16-proof.json`, which can be used directly for on-chain verification:

```bash
npm run verify
```

---
presentation:
  theme: black.css
  enableAppView: true
  slideNumber: true
---

# GCP Pub/Sub 與 Prover Service 部署指南

本指南將協助您將目前的 Pub/Sub 架構與 Prover Service 正式部署至 Google Cloud Platform (GCP)。

---

## 步驟一：GCP 專案與 Pub/Sub 資源設定

在部署程式碼之前，必須先在 GCP 上建立必要的基礎設施。

### 1. 準備工作

確保您已安裝 `gcloud` CLI 並完成登入：

```bash
gcloud auth login --no-launch-browser  
gcloud config set project [YOUR_PROJECT_ID]
```

### 2. 啟用必要 API

```bash
gcloud services enable pubsub.googleapis.com
gcloud services enable compute.googleapis.com
```

等待幾分鐘，直到以下回應出現：

``` txt
Operation "operations/acf.p2-210193831987-0e03b4ff-f8ba-46cb-be76-f9adc4fed906" finished successfully.
```

### 3. 建立 Service Account (SA)

Prover Service 需要專用的身分來存取 Pub/Sub。

```bash
# 建立 Service Account
gcloud iam service-accounts create prover-service-sa \
    --display-name="Prover Service Account"

# 賦予權限 (Subscriber 接收任務, Publisher 發送結果)
gcloud projects add-iam-policy-binding [YOUR_PROJECT_ID] \
    --member="serviceAccount:prover-service-sa@[YOUR_PROJECT_ID].iam.gserviceaccount.com" \
    --role="roles/pubsub.subscriber"

gcloud projects add-iam-policy-binding [YOUR_PROJECT_ID] \
    --member="serviceAccount:prover-service-sa@[YOUR_PROJECT_ID].iam.gserviceaccount.com" \
    --role="roles/pubsub.publisher"
```

### 4. 建立 Topics 與 Subscriptions

```bash
# 建立 Topics
gcloud pubsub topics create prover-requests
gcloud pubsub topics create prover-results

# 建立 Subscriptions
# 注意：ack-deadline 設為 600s 是因為證明生成時間較長
gcloud pubsub subscriptions create prover-requests-sub \
    --topic=prover-requests \
    --ack-deadline=600

gcloud pubsub subscriptions create prover-results-sub \
    --topic=prover-results
```

**驗證資源已建立：**

```bash
gcloud pubsub topics list
gcloud pubsub subscriptions list
```

---

## 步驟二：準備 VM (Compute Engine)

我們將建立一台運算優化型 (Compute-optimized) 的 VM 來運行 Prover。

### 1. 選擇機器規格

* **系列**: C2 (Compute-optimized)
* **建議規格**: `c2-standard-8` (8 vCPU, 32GB RAM)
* **磁碟**: 至少 50GB SSD (PD-SSD)

### 2. 建立 VM

使用以下指令建立 VM，並綁定剛才建立的 Service Account，等待約 1-2 分鐘讓 VM 啟動完成。

* `--preemptible` (可選)：使用 Spot VM (較便宜，但可能會被中斷，適合無狀態的 Prover)。

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

## 步驟三：部署 Prover Service 到 VM

### 1. 建置 Docker Image (在本機)

使用 Docker 可避免在 VM 上安裝 Rust 編譯環境，專案根目錄已有 Dockerfile。

```bash
gcloud auth configure-docker

# 建置 Image (請將 [YOUR_PROJECT_ID] 替換)
docker build -t gcr.io/[YOUR_PROJECT_ID]/prover-service:latest .

# 推送
docker push gcr.io/[YOUR_PROJECT_ID]/prover-service:latest
```

### 2. 在 VM 上運行

SSH 進入您的 VM（首次連線會在本機 `~/.ssh/` 產生 SSH 金鑰）：

```bash
gcloud compute ssh prover-instance-1
```

在 VM 內執行：

#### A. 安裝 Docker

```bash
sudo apt-get update
sudo apt-get install -y docker.io
sudo usermod -aG docker $USER
# 登出再登入以生效
exit
gcloud compute ssh prover-instance-1
```

#### B. 設定認證並拉取 Image

```bash
gcloud auth configure-docker
```

#### C. 啟動服務

使用環境變數設定正式環境參數。

* `MAX_CONCURRENT_PROOFS`: 根據您的 VM 規格調整 (c2-standard-8 建議設為 1 或 2)。

```bash
docker run -d \
  --name prover-service \
  --restart always \
  -e GCP_PROJECT_ID=[YOUR_PROJECT_ID] \
  -e PROVER_SUBSCRIPTION=prover-requests-sub \
  -e RESULT_TOPIC=prover-results \
  -e MAX_CONCURRENT_PROOFS=2 \
  -e PROOF_TIMEOUT_SECS=3600 \
  -e RUST_LOG=info \
  gcr.io/[YOUR_PROJECT_ID]/prover-service:latest
```

---

## 步驟四：驗證與監控

### 1. 查看 Logs

在 VM 上查看 Prover 是否正常啟動：

```bash
docker logs -f prover-service
```

您應該看到 `Starting prover service, subscribing to 'prover-requests-sub'` 的訊息。

### 2. 發送測試請求

在您的本機電腦執行測試腳本，發送請求到**真實的 GCP Pub/Sub**。

修改 `.env` (或確保環境變數設定正確)：

```bash
GCP_PROJECT_ID=[YOUR_PROJECT_ID]
# 確保此行被註解
# PUBSUB_EMULATOR_HOST=localhost:8085
```

執行發布：

```bash
npx tsx scripts/test-pubsub.ts publish normal
```

### 3. 確認結果

回到 VM 的 log 視窗，您應該會看到它收到訊息並開始 `Processing proof request`。稍後，它會顯示 `Proof generated successfully` 並將結果發布回 `prover-results`。

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
gcloud services enable artifactregistry.googleapis.com
```

等待幾分鐘，直到以下回應出現：

``` txt
Operation "operations/acf.p2-210193831987-0e03b4ff-f8ba-46cb-be76-f9adc4fed906" finished successfully.
```

### 3. 建立 Service Account (SA)

Prover Service 需要專用的身分來存取 Pub/Sub 和 Artifact Registry。

```bash
# 建立 Service Account
gcloud iam service-accounts create prover-service-sa \
    --display-name="Prover Service Account"

# 賦予權限
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

### 1. 在本機建置 Docker Image

#### A. 設定 GCR 認證

```bash
gcloud auth configure-docker
```

#### B. 建置 Image

由於 VM 是 x86_64 架構，macOS (ARM64) 用戶需要指定目標平台：

```bash
docker build --platform linux/amd64 -t gcr.io/[YOUR_PROJECT_ID]/prover-service:latest .
```

需要大概 30-60 分鐘完成建置。

#### C. 推送 Image

```bash
docker push gcr.io/[YOUR_PROJECT_ID]/prover-service:latest
```

根據網速，需要大概 1 小時。

### 2. 在 VM 上運行

SSH 進入您的 VM（首次連線會在本機 `~/.ssh/` 產生 SSH 金鑰）：

```bash
gcloud compute ssh prover-instance-1
```

在 VM 內執行：

#### A. 安裝 Docker

使用官方 Docker repository 安裝最新版本（避免 Docker API 版本不相容問題）：

```bash
sudo apt-get update
sudo apt-get install -y ca-certificates curl gnupg

# 新增 Docker 官方 GPG key
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
sudo chmod a+r /etc/apt/keyrings/docker.gpg

# 新增 Docker repository
echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
  $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
  sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

# 安裝 Docker
sudo apt-get update
sudo apt-get install -y docker-ce docker-ce-cli containerd.io

sudo usermod -aG docker $USER
# 登出再登入以生效
exit
gcloud compute ssh prover-instance-1
```

#### B. 設定 GCR 認證

```bash
gcloud auth configure-docker
```

#### C. 準備 Host 目錄（Docker-in-Docker 必要步驟）

Prover Service 內部會呼叫另一個 Docker container (`pico_gnark_cli`) 來生成 Groth16 proof。由於我們掛載了 Docker socket，內層 container 是由 Host 的 Docker daemon 執行的，因此需要在 Host 上準備共享目錄：

```bash
# 建立 Host 上的資料目錄
sudo mkdir -p /app/data

# 從 Image 中複製 Groth16 setup 檔案到 Host
docker run --rm \
  -v /app/data:/host-data \
  gcr.io/[YOUR_PROJECT_ID]/prover-service:latest \
  cp /app/data/vm_pk /app/data/vm_vk /host-data/

# 確認檔案已複製（vm_pk 約 1.3GB，vm_vk 約 520 bytes）
ls -la /app/data/
```

#### D. 設定 Application Default Credentials (ADC)

有時 Container 啟動後出現 `PermissionDenied` 錯誤，表示 Rust Pub/Sub client 無法正確使用 GCE metadata 認證。為避免這種狀況，需要手動設定 Application Default Credentials (ADC)：

```bash
# 在 VM 上執行，會給你一個 URL
gcloud auth application-default login --no-launch-browser
```

在本機瀏覽器打開該 URL，登入 Google 帳號後，將授權碼貼回 VM 終端機。
這會在 VM 上的 `~/.config/gcloud/application_default_credentials.json` 產生 ADC 認證檔案。

#### E. 啟動服務

使用環境變數設定正式環境參數。

* `MAX_CONCURRENT_PROOFS`: 根據您的 VM 規格調整 (c2-standard-8 建議設為 1 或 2)。

```bash
# 抓取最新版 Image (如果有更新)
docker pull gcr.io/[YOUR_PROJECT_ID]/prover-service:latest

# 停止並移除舊的 Container（如果有的話）
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

**Note**：

* `-v /app/data:/app/data` 掛載 Host 資料目錄（Docker-in-Docker 必要），確保 Host 和 Container 使用相同路徑，讓 Docker daemon 能正確找到 Host 上的目錄和檔案。
* `-v ~/.config/gcloud/application_default_credentials.json:/app/credentials.json:ro` 掛載 ADC 認證檔案
* `-e GOOGLE_APPLICATION_CREDENTIALS=/app/credentials.json` 告訴 SDK 認證檔案位置

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
export $(grep -v '^#' .env | xargs)
npm run pubsub:publish normal
```

### 3. 確認結果

回到 VM 的 log 視窗，您應該會看到它收到訊息並開始 `Processing proof request`。稍後，它會顯示 `Proof generated successfully` 並將結果發布回 `prover-results`。

### 4. 接收 Proof 結果

Prover Service 會將產生的 proof 發布到 `prover-results` Topic。以下是接收結果的方式：

#### A. 使用 gcloud CLI（測試用）

一次性拉取訊息：

```bash
gcloud pubsub subscriptions pull prover-results-sub --auto-ack
```

持續監聽（每秒輪詢）：

```bash
while true; do
  gcloud pubsub subscriptions pull prover-results-sub --auto-ack --format="json"
  sleep 1
done
```

> **Note**: `--auto-ack` 會在拉取後自動確認訊息，訊息將從佇列中移除。若不加此參數，訊息會在 ack deadline 後重新出現。

#### B. 使用專案內建腳本（推薦）

專案已內建 TypeScript subscriber，可以持續監聽結果：

```bash
export $(grep -v '^#' .env | xargs)
npm run pubsub:listen forever
```

收到成功的 proof 後，腳本會自動將結果儲存到 `prover/data/groth16-proof.json`，可直接用於鏈上驗證：

```bash
npm run verify
```

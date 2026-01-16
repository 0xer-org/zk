#!/bin/bash
# Start Google Cloud Pub/Sub Emulator using Docker for local testing

CONTAINER_NAME="pubsub-emulator"
PROJECT_ID="test-project"
HOST_PORT="8085"

echo "🚀 Starting Pub/Sub Emulator (Docker)..."
echo "   Project: $PROJECT_ID"
echo "   Host: localhost:$HOST_PORT"
echo ""

# Stop and remove existing container if it exists
if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "📦 Removing existing container..."
    docker rm -f $CONTAINER_NAME > /dev/null 2>&1
fi

echo "🐳 Starting Docker container..."
docker run -d \
    --name $CONTAINER_NAME \
    -p $HOST_PORT:8085 \
    -e PUBSUB_PROJECT_ID=$PROJECT_ID \
    gcr.io/google.com/cloudsdktool/google-cloud-cli:emulators \
    gcloud beta emulators pubsub start \
        --project=$PROJECT_ID \
        --host-port=0.0.0.0:8085

# Wait for container to start
sleep 2

# Check if container is running
if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo ""
    echo "✅ Pub/Sub Emulator is running!"
    echo "   Container: $CONTAINER_NAME"
    echo "   Endpoint: localhost:$HOST_PORT"
    echo ""
    echo "To stop: docker stop $CONTAINER_NAME"
    echo "To view logs: docker logs -f $CONTAINER_NAME"
else
    echo ""
    echo "❌ Failed to start emulator. Check logs with:"
    echo "   docker logs $CONTAINER_NAME"
    exit 1
fi

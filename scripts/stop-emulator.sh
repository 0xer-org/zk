#!/bin/bash
# Stop Google Cloud Pub/Sub Emulator Docker container

CONTAINER_NAME="pubsub-emulator"

echo "🛑 Stopping Pub/Sub Emulator..."

if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    docker stop $CONTAINER_NAME > /dev/null 2>&1
    docker rm $CONTAINER_NAME > /dev/null 2>&1
    echo "✅ Emulator stopped and container removed"
else
    echo "⚠️  Emulator is not running"
fi

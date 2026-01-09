#!/bin/bash
# Rebuild BuildScale Docker image with git commit information

set -e

cd "$(dirname "$0")"

# Get current git commit hash and timestamp
BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
GIT_COMMIT=$(git rev-parse --short HEAD)

echo "Building Docker image..."
echo "  BUILD_DATE: $BUILD_DATE"
echo "  GIT_COMMIT: $GIT_COMMIT"
echo ""

# Build with arguments
BUILD_DATE="$BUILD_DATE" GIT_COMMIT="$GIT_COMMIT" docker-compose build

# Stop and remove buildscale container before starting
echo ""
echo "Stopping buildscale container..."
docker-compose down buildscale

# Start containers
echo ""
echo "Starting containers..."
docker-compose up -d buildscale

# Show logs
echo ""
echo "Server logs:"
sleep 2
docker logs buildscale 2>&1 | grep "server listening"

# Show git info to verify build matches current state
echo ""
echo "Git status:"
git log -1 --oneline
git status --short
echo ""

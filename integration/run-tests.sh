#!/bin/bash
set -e

echo "=== Starting Pesto Integration Tests ==="
echo ""

# Clean up any existing containers
echo "Cleaning up existing containers..."
docker compose down -v 2>/dev/null || true

# Start the environment
echo "Starting services..."
docker compose up -d --build --force-recreate --renew-anon-volumes

# Wait for Pesto to be ready
echo "Waiting for Pesto to be ready..."
for i in {1..30}; do
    if curl -s http://127.0.0.1:8080/metrics > /dev/null 2>&1; then
        echo "Pesto is ready!"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "Timeout waiting for Pesto"
        docker compose logs pesto
        exit 1
    fi
    sleep 1
done

# Wait for sFlow producer to finish
echo "Waiting for sFlow producer to send datagrams..."
sleep 15

# Run verification
echo ""
./tests/verify.sh

# Show logs if verification failed
if [ $? -ne 0 ]; then
    echo ""
    echo "=== Pesto Logs ==="
    docker compose logs pesto
    echo ""
    echo "=== sFlow Producer Logs ==="
    docker compose logs sflow-producer
fi

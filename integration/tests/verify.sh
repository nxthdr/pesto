#!/bin/bash
set -e

echo "=== Pesto Integration Test Verification ==="
echo ""

# Wait for services to be ready
echo "Waiting for services to start..."
sleep 10

# Check Pesto metrics
echo "1. Checking Pesto metrics..."
DATAGRAMS=$(curl -s http://127.0.0.1:8080/metrics | grep 'pesto_sflow_datagrams_total{status="success"}' | awk '{print $2}')
SAMPLES=$(curl -s http://127.0.0.1:8080/metrics | grep 'pesto_sflow_samples_total' | awk '{print $2}')
RECORDS=$(curl -s http://127.0.0.1:8080/metrics | grep 'pesto_sflow_records_total' | awk '{print $2}')
KAFKA_SUCCESS=$(curl -s http://127.0.0.1:8080/metrics | grep 'pesto_kafka_messages_total{status="success"}' | awk '{print $2}')

echo "   Datagrams received: ${DATAGRAMS:-0}"
echo "   Samples processed: ${SAMPLES:-0}"
echo "   Records transmitted: ${RECORDS:-0}"
echo "   Kafka messages sent: ${KAFKA_SUCCESS:-0}"
echo ""

# Check ClickHouse data
echo "2. Checking ClickHouse data..."
sleep 5  # Give ClickHouse time to consume from Kafka

FLOWS=$(docker exec integration-clickhouse-1 clickhouse-client --query "SELECT COUNT(*) FROM sflow.flows" 2>/dev/null || echo "0")
echo "   Flows in ClickHouse: $FLOWS"

if [ "$FLOWS" -gt 0 ]; then
    echo ""
    echo "3. Flow records:"
    docker exec integration-clickhouse-1 clickhouse-client --query "SELECT time_received_ns, sampler_address, src_addr, dst_addr, src_port, dst_port, protocol, bytes, packets FROM sflow.flows LIMIT 3 FORMAT Vertical"
fi

echo ""
echo "=== Test Summary ==="
if [ "${DATAGRAMS:-0}" -gt 0 ] && [ "${FLOWS:-0}" -gt 0 ]; then
    echo "✅ SUCCESS: Data flowing from sFlow producer → Pesto → Kafka → ClickHouse"
    exit 0
else
    echo "❌ FAILURE: Data pipeline incomplete"
    echo "   Check logs with: docker compose logs"
    exit 1
fi

# Integration Tests

This setup provides an environment for integration testing of Pesto's sFlow collection and Kafka production capabilities.

## Components

- **sFlow Producer:** A simple UDP client that reads `.bin` files containing sFlow datagrams and sends them to Pesto
- **Pesto:** The sFlow collector under test, listening for sFlow datagrams on UDP port 6343
- **Redpanda:** A Kafka instance serving as the message broker for records produced by Pesto
- **ClickHouse:** A database used to consume and store the messages from Redpanda for verification

## Data Flow

1. The sFlow producer reads test sFlow datagrams from `.bin` files in the `tests/` directory
2. The producer sends these datagrams via UDP to Pesto (`10.0.0.99:6343`)
3. Pesto parses the sFlow datagrams and serializes them to Cap'n Proto format
4. Pesto produces the serialized records to the `pesto-sflow` topic in Redpanda
5. ClickHouse uses a table based on the [Kafka engine](https://clickhouse.com/docs/en/engines/table-engines/integrations/kafka) (`sflow.from_kafka`) to read messages from the Redpanda topic
6. The consumed data is inserted into a storage table (`sflow.records`) for analysis and verification

## Test Data

The test sFlow datagrams are located in the `tests/` directory:
- `sflow.bin`: Sample sFlow v5 datagram from the sflow-parser test suite

## Usage

### Start the environment

```sh
docker compose up -d --build --force-recreate --renew-anon-volumes
```

### Check Prometheus metrics

```sh
curl -s http://127.0.0.1:8080/metrics | grep pesto
```

Expected metrics:
- `pesto_sflow_datagrams_total`: Number of datagrams received
- `pesto_sflow_samples_total`: Number of samples processed
- `pesto_sflow_records_total`: Number of records sent to Kafka
- `pesto_kafka_messages_total`: Number of Kafka messages produced

### Interact with Redpanda

Check consumer group status:
```sh
docker exec -ti integration-redpanda-1 rpk group describe clickhouse-pesto-group
```

List topics:
```sh
docker exec -ti integration-redpanda-1 rpk topic list
```

### Check ClickHouse tables

View all records:
```sh
docker exec -ti integration-clickhouse-1 clickhouse-client --query "SELECT * FROM sflow.records FORMAT Pretty"
```

Count records by sample type:
```sh
docker exec -ti integration-clickhouse-1 clickhouse-client --query "SELECT sample_type, count() FROM sflow.records GROUP BY sample_type"
```

View flow samples:
```sh
docker exec -ti integration-clickhouse-1 clickhouse-client --query "SELECT time_received, agent_addr, sample_type, sampling_rate, source_id FROM sflow.records WHERE sample_type LIKE '%Flow%' FORMAT Pretty"
```

View counter samples:
```sh
docker exec -ti integration-clickhouse-1 clickhouse-client --query "SELECT time_received, agent_addr, sample_type, source_id FROM sflow.records WHERE sample_type LIKE '%Counter%' FORMAT Pretty"
```

View records with details:
```sh
docker exec -ti integration-clickhouse-1 clickhouse-client --query "SELECT time_received, agent_addr, sample_type, sampling_rate, sample_pool, drops, input_interface, output_interface FROM sflow.records FORMAT Vertical"
```

### Send additional test data

You can manually trigger the sFlow producer to send more data:

```sh
docker compose run --rm sflow-producer --file /data/sflow.bin --target 10.0.0.99:6343 --count 5 --interval 1000
```

### View logs

Pesto logs:
```sh
docker compose logs -f pesto
```

All services:
```sh
docker compose logs -f
```

### Stop the environment

```sh
docker compose down
```

## Adding New Test Cases

To add new test scenarios:

1. Place your `.bin` file containing sFlow datagrams in the `tests/` directory
2. Update the `sflow-producer` command in `compose.yml` to use your new file
3. Restart the environment

## Troubleshooting

### No data in ClickHouse

1. Check if Pesto received the datagrams:
   ```sh
   curl -s http://127.0.0.1:8080/metrics | grep pesto_sflow_datagrams_total
   ```

2. Check if Kafka messages were produced:
   ```sh
   curl -s http://127.0.0.1:8080/metrics | grep pesto_kafka_messages_total
   ```

3. Check Redpanda topic:
   ```sh
   docker exec -ti integration-redpanda-1 rpk topic describe pesto-sflow
   ```

4. Check ClickHouse Kafka consumer:
   ```sh
   docker exec -ti integration-clickhouse-1 clickhouse-client --query "SELECT * FROM system.kafka_consumers"
   ```

### Parse errors

Check Pesto logs for parse errors:
```sh
docker compose logs pesto | grep -i error
```

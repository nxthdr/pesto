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
3. Pesto parses the sFlow datagrams using the `sflow-parser` library
4. **Pesto sends one Kafka message per flow record** (flattened structure)
5. Each message contains: datagram metadata, sample metadata, and flow data (IPs, ports, protocol)
6. IPv4 addresses are converted to IPv6-mapped format for uniform handling
7. ClickHouse consumes messages using the Kafka engine table (`sflow.from_kafka`)
8. A materialized view (`sflow.from_kafka_mv`) extracts and stores flow data in `sflow.flows`
9. The `sflow.flows` table contains complete flow information matching the infrastructure flows schema

## Cap'n Proto Schema

The sFlow records are serialized using a **flattened Cap'n Proto structure** (`SFlowFlowRecord`):

**Design principle**: One message per flow record instead of nested arrays

**Message structure**:
- **Datagram metadata**: timestamp, agent address/port, sequence numbers, uptime
- **Sample metadata**: source ID, sampling rate, sample pool, drops, interfaces
- **Flow data**: IPv6 addresses (IPv4-mapped), ports, protocol, packet length

**Key features**:
- All IP addresses stored as IPv6 (IPv4 converted to IPv6-mapped format)
- No nested unions or arrays - completely flat structure
- Easy to consume by ClickHouse without complex SQL

The schema is located in `../schemas/sflow.capnp` and is mounted into ClickHouse at `/var/lib/clickhouse/format_schemas/`.

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

View all flows:
```sh
docker exec -ti integration-clickhouse-1 clickhouse-client --query "SELECT * FROM sflow.flows FORMAT Pretty"
```

Count total flows:
```sh
docker exec -ti integration-clickhouse-1 clickhouse-client --query "SELECT count() FROM sflow.flows"
```

View flows with computed traffic (multiply by sampling rate):
```sh
docker exec -ti integration-clickhouse-1 clickhouse-client --query "SELECT time_received_ns, IPv6NumToString(src_addr) as src, IPv6NumToString(dst_addr) as dst, src_port, dst_port, protocol, bytes, packets, sampling_rate, bytes * sampling_rate as total_bytes FROM sflow.flows LIMIT 10 FORMAT Vertical"
```

Aggregate traffic by time:
```sh
docker exec -ti integration-clickhouse-1 clickhouse-client --query "SELECT toStartOfMinute(time_received_ns) as time, sum(bytes * max2(sampling_rate, 1)) as total_bytes FROM sflow.flows GROUP BY time ORDER BY time FORMAT Pretty"
```

Check Kafka consumer status:
```sh
docker exec -ti integration-clickhouse-1 clickhouse-client --query "SELECT database, table, num_messages_read, last_commit_time FROM system.kafka_consumers FORMAT Vertical"
```

Show table schema:
```sh
docker exec -ti integration-clickhouse-1 clickhouse-client --query "DESCRIBE sflow.flows"
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

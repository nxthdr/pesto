-- Create database for sFlow data
CREATE DATABASE IF NOT EXISTS sflow;

-- Create Kafka consumer table for flat sFlow flow records
CREATE TABLE IF NOT EXISTS sflow.from_kafka
(
    timeReceivedNs UInt64,
    agentAddr FixedString(16),
    agentPort UInt16,
    agentSubId UInt32,
    datagramSequenceNumber UInt32,
    uptime UInt32,
    sampleSequenceNumber UInt32,
    sourceId UInt32,
    samplingRate UInt32,
    samplePool UInt32,
    drops UInt32,
    inputInterface UInt32,
    outputInterface UInt32,
    length UInt32,
    protocol UInt32,
    srcIp FixedString(16),
    dstIp FixedString(16),
    srcPort UInt32,
    dstPort UInt32,
    tcpFlags UInt32,
    tos UInt32
)
ENGINE = Kafka()
SETTINGS
    kafka_broker_list = '10.0.0.100:9092',
    kafka_topic_list = 'pesto-sflow',
    kafka_group_name = 'clickhouse-pesto-group',
    kafka_format = 'CapnProto',
    kafka_schema = 'sflow:SFlowFlowRecord',
    kafka_num_consumers = 1,
    kafka_max_block_size = 1048576;

-- Create storage table for sFlow flows
CREATE TABLE IF NOT EXISTS sflow.flows
(
    date Date,
    time_inserted_ns DateTime64(9),
    time_received_ns DateTime64(9),
    sequence_num UInt32,
    sampling_rate UInt64,
    sampler_address IPv6,
    sampler_port UInt16,
    src_addr IPv6,
    dst_addr IPv6,
    src_port UInt32,
    dst_port UInt32,
    protocol UInt32,
    etype UInt32,
    packet_length UInt32,
    bytes UInt64,
    packets UInt64
)
ENGINE = MergeTree()
PARTITION BY date
ORDER BY (time_received_ns, src_addr, dst_addr)
TTL date + INTERVAL 7 DAY DELETE;

-- Create helper function to convert FixedString(16) to IPv6
CREATE FUNCTION IF NOT EXISTS convertToIPv6 AS (addr) ->
(
    if(reinterpretAsUInt128(substring(reverse(addr), 1, 12)) = 0,
       IPv4ToIPv6(reinterpretAsUInt32(substring(reverse(addr), 13, 4))),
       toIPv6(addr))
);

-- Create materialized view to flatten flow data
CREATE MATERIALIZED VIEW IF NOT EXISTS sflow.from_kafka_mv TO sflow.flows
AS SELECT
    toDate(timeReceivedNs / 1000000000) AS date,
    now() AS time_inserted_ns,
    toDateTime64(timeReceivedNs / 1000000000, 9) AS time_received_ns,
    datagramSequenceNumber AS sequence_num,
    toUInt64(samplingRate) AS sampling_rate,
    toIPv6(agentAddr) AS sampler_address,
    agentPort AS sampler_port,

    -- Extract IPs (already IPv6 format)
    toIPv6(srcIp) AS src_addr,
    toIPv6(dstIp) AS dst_addr,

    -- Extract ports and protocol
    srcPort AS src_port,
    dstPort AS dst_port,
    protocol AS protocol,
    0 AS etype,

    -- Raw packet data
    length AS packet_length,
    toUInt64(length) AS bytes,
    1 AS packets
FROM sflow.from_kafka
WHERE samplingRate > 0;

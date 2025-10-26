-- Create database for sFlow data
CREATE DATABASE IF NOT EXISTS sflow;

-- Create Kafka consumer table
CREATE TABLE IF NOT EXISTS sflow.from_kafka
(
    timeReceivedNs UInt64,
    agentAddr FixedString(16),
    agentPort UInt16,
    agentSubId UInt32,
    sequenceNumber UInt32,
    uptime UInt32,
    sampleType UInt16,
    sourceId UInt32,
    samplingRate UInt32,
    samplePool UInt32,
    drops UInt32,
    inputInterface UInt32,
    outputInterface UInt32,
    sample String
)
ENGINE = Kafka()
SETTINGS
    kafka_broker_list = '10.0.0.100:9092',
    kafka_topic_list = 'pesto-sflow',
    kafka_group_name = 'clickhouse-pesto-group',
    kafka_format = 'CapnProto',
    kafka_schema = 'sflow:SFlowRecord',
    kafka_num_consumers = 1,
    kafka_max_block_size = 1048576;

-- Create storage table for sFlow records
CREATE TABLE IF NOT EXISTS sflow.records
(
    date Date,
    time_inserted_ns DateTime64(9),
    time_received_ns DateTime64(9),
    agent_addr IPv6,
    agent_port UInt16,
    agent_sub_id UInt32,
    sequence_number UInt32,
    uptime UInt32,
    sample_type String,
    source_id UInt32,
    sampling_rate UInt32,
    sample_pool UInt32,
    drops UInt32,
    input_interface UInt32,
    output_interface UInt32,
    sample String
)
ENGINE = MergeTree()
PARTITION BY date
ORDER BY (agent_addr, time_received_ns)
TTL date + INTERVAL 7 DAY DELETE;

-- Create helper function to convert FixedString(16) to IPv6
CREATE FUNCTION IF NOT EXISTS convertToIPv6 AS (addr) ->
(
    if(reinterpretAsUInt128(substring(reverse(addr), 1, 12)) = 0, 
       IPv4ToIPv6(reinterpretAsUInt32(substring(reverse(addr), 13, 4))), 
       addr)
);

-- Create materialized view to consume from Kafka
CREATE MATERIALIZED VIEW IF NOT EXISTS sflow.from_kafka_mv TO sflow.records
AS SELECT
    toDate(timeReceivedNs / 1000000000) AS date,
    now() AS time_inserted_ns,
    toDateTime64(timeReceivedNs / 1000000000, 9) AS time_received_ns,
    convertToIPv6(agentAddr) AS agent_addr,
    agentPort AS agent_port,
    agentSubId AS agent_sub_id,
    sequenceNumber AS sequence_number,
    uptime,
    CASE sampleType
        WHEN 0 THEN 'FlowSample'
        WHEN 1 THEN 'CounterSample'
        WHEN 2 THEN 'ExpandedFlowSample'
        WHEN 3 THEN 'ExpandedCounterSample'
        ELSE 'Unknown'
    END AS sample_type,
    sourceId AS source_id,
    samplingRate AS sampling_rate,
    samplePool AS sample_pool,
    drops,
    inputInterface AS input_interface,
    outputInterface AS output_interface,
    sample
FROM sflow.from_kafka;

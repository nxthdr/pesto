@0xb8c3d4e5f6a7b8c9;

# Flat sFlow flow record - ClickHouse compatible (no unions)
# Following goflow2's approach: only flow samples, no counter samples
struct SFlowFlowRecord {
  # Datagram metadata
  timeReceivedNs @0 :UInt64;
  agentAddr @1 :Data;  # IPv4 or IPv6 address (16 bytes)
  agentPort @2 :UInt16;
  agentSubId @3 :UInt32;
  datagramSequenceNumber @4 :UInt32;
  uptime @5 :UInt32;

  # Sample metadata
  sampleSequenceNumber @6 :UInt32;
  sourceId @7 :UInt32;

  # Flow sample fields
  samplingRate @8 :UInt32;
  samplePool @9 :UInt32;
  drops @10 :UInt32;
  inputInterface @11 :UInt32;
  outputInterface @12 :UInt32;

  # Flow data - always IPv6 (IPv4 mapped to IPv6)
  length @13 :UInt32;
  protocol @14 :UInt32;
  srcIp @15 :Data;  # IPv6 address (16 bytes)
  dstIp @16 :Data;  # IPv6 address (16 bytes)
  srcPort @17 :UInt32;
  dstPort @18 :UInt32;
  tcpFlags @19 :UInt32;
  tos @20 :UInt32;
}


@0xb8c3d4e5f6a7b8c9;

# Flattened sFlow flow record - one message per flow record
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
  samplingRate @8 :UInt32;
  samplePool @9 :UInt32;
  drops @10 :UInt32;
  inputInterface @11 :UInt32;
  outputInterface @12 :UInt32;
  
  # Flow record data - separate optional fields for each type
  sampledIpv4 @13 :SampledIpv4;
  sampledIpv6 @14 :SampledIpv6;
  extendedSwitch @15 :ExtendedSwitch;
  extendedRouter @16 :ExtendedRouter;
  extendedGateway @17 :ExtendedGateway;
}

struct FlowSample {
  sequenceNumber @0 :UInt32;
  sourceId @1 :UInt32;
  samplingRate @2 :UInt32;
  samplePool @3 :UInt32;
  drops @4 :UInt32;
  inputInterface @5 :UInt32;
  outputInterface @6 :UInt32;
  flowRecords @7 :List(FlowRecord);
}

struct FlowSampleExpanded {
  sequenceNumber @0 :UInt32;
  sourceIdType @1 :UInt32;
  sourceIdIndex @2 :UInt32;
  samplingRate @3 :UInt32;
  samplePool @4 :UInt32;
  drops @5 :UInt32;
  inputInterfaceFormat @6 :UInt32;
  inputInterfaceValue @7 :UInt32;
  outputInterfaceFormat @8 :UInt32;
  outputInterfaceValue @9 :UInt32;
  flowRecords @10 :List(FlowRecord);
}

struct CounterSample {
  sequenceNumber @0 :UInt32;
  sourceId @1 :UInt32;
  counterRecords @2 :List(CounterRecord);
}

struct CounterSampleExpanded {
  sequenceNumber @0 :UInt32;
  sourceIdType @1 :UInt32;
  sourceIdIndex @2 :UInt32;
  counterRecords @3 :List(CounterRecord);
}

struct FlowRecord {
  union {
    # Raw packet header - Format (0,1)
    rawPacketHeader @0 :RawPacketHeader;
    # Ethernet frame - Format (0,2)
    ethernetFrame @1 :EthernetFrame;
    # IPv4 data - Format (0,3)
    sampledIpv4 @2 :SampledIpv4;
    # IPv6 data - Format (0,4)
    sampledIpv6 @3 :SampledIpv6;
    # Extended switch data - Format (0,1001)
    extendedSwitch @4 :ExtendedSwitch;
    # Extended router data - Format (0,1002)
    extendedRouter @5 :ExtendedRouter;
    # Extended gateway data - Format (0,1003)
    extendedGateway @6 :ExtendedGateway;
    # Extended user - Format (0,1004)
    extendedUser @7 :ExtendedUser;
    # Extended URL - Format (0,1005)
    extendedUrl @8 :ExtendedUrl;
  }
}

struct RawPacketHeader {
  protocol @0 :UInt32;
  frameLength @1 :UInt32;
  stripped @2 :UInt32;
  headerLength @3 :UInt32;
  header @4 :Data;
}

struct EthernetFrame {
  srcMac @0 :Data;  # 6 bytes
  dstMac @1 :Data;  # 6 bytes
  ethType @2 :UInt32;
}

struct SampledIpv4 {
  length @0 :UInt32;
  protocol @1 :UInt32;
  srcIp @2 :Data;  # 4 bytes
  dstIp @3 :Data;  # 4 bytes
  srcPort @4 :UInt32;
  dstPort @5 :UInt32;
  tcpFlags @6 :UInt32;
  tos @7 :UInt32;
}

struct SampledIpv6 {
  length @0 :UInt32;
  protocol @1 :UInt32;
  srcIp @2 :Data;  # 16 bytes
  dstIp @3 :Data;  # 16 bytes
  srcPort @4 :UInt32;
  dstPort @5 :UInt32;
  tcpFlags @6 :UInt32;
  priority @7 :UInt32;
}

struct ExtendedSwitch {
  srcVlan @0 :UInt32;
  srcPriority @1 :UInt32;
  dstVlan @2 :UInt32;
  dstPriority @3 :UInt32;
}

struct ExtendedRouter {
  nextHop @0 :Data;  # IPv4 or IPv6 (16 bytes)
  srcMaskLen @1 :UInt32;
  dstMaskLen @2 :UInt32;
}

struct ExtendedGateway {
  nextHop @0 :Data;  # IPv4 or IPv6 (16 bytes)
  as @1 :UInt32;
  srcAs @2 :UInt32;
  srcPeerAs @3 :UInt32;
  asPath @4 :List(UInt32);
  communities @5 :List(UInt32);
  localPref @6 :UInt32;
}

struct ExtendedUser {
  srcCharset @0 :UInt32;
  srcUser @1 :Text;
  dstCharset @2 :UInt32;
  dstUser @3 :Text;
}

struct ExtendedUrl {
  direction @0 :UInt32;
  url @1 :Text;
  host @2 :Text;
}

struct CounterRecord {
  union {
    # Generic Interface Counters - Format (0,1)
    genericInterface @0 :GenericInterfaceCounters;
    # Ethernet Interface Counters - Format (0,2)
    ethernetInterface @1 :EthernetInterfaceCounters;
    # Processor Counters - Format (0,1001)
    processor @2 :ProcessorCounters;
  }
}

struct GenericInterfaceCounters {
  index @0 :UInt32;
  ifType @1 :UInt32;
  speed @2 :UInt64;
  direction @3 :UInt32;
  status @4 :UInt32;
  inOctets @5 :UInt64;
  inUnicastPkts @6 :UInt32;
  inMulticastPkts @7 :UInt32;
  inBroadcastPkts @8 :UInt32;
  inDiscards @9 :UInt32;
  inErrors @10 :UInt32;
  inUnknownProtos @11 :UInt32;
  outOctets @12 :UInt64;
  outUnicastPkts @13 :UInt32;
  outMulticastPkts @14 :UInt32;
  outBroadcastPkts @15 :UInt32;
  outDiscards @16 :UInt32;
  outErrors @17 :UInt32;
  promiscuousMode @18 :UInt32;
}

struct EthernetInterfaceCounters {
  alignmentErrors @0 :UInt32;
  fcsErrors @1 :UInt32;
  singleCollisionFrames @2 :UInt32;
  multipleCollisionFrames @3 :UInt32;
  sqeTestErrors @4 :UInt32;
  deferredTransmissions @5 :UInt32;
  lateCollisions @6 :UInt32;
  excessiveCollisions @7 :UInt32;
  internalMacTransmitErrors @8 :UInt32;
  carrierSenseErrors @9 :UInt32;
  frameTooLongs @10 :UInt32;
  internalMacReceiveErrors @11 :UInt32;
  symbolErrors @12 :UInt32;
}

struct ProcessorCounters {
  cpu5s @0 :UInt32;  # 5 second average CPU utilization
  cpu1m @1 :UInt32;  # 1 minute average CPU utilization
  cpu5m @2 :UInt32;  # 5 minute average CPU utilization
  totalMemory @3 :UInt64;
  freeMemory @4 :UInt64;
}

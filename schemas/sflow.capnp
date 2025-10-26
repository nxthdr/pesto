@0xb8c3d4e5f6a7b8c9;

struct SFlowRecord {
  # Metadata
  timeReceivedNs @0 :UInt64;
  agentAddr @1 :Data;  # IPv4 or IPv6 address (16 bytes)
  agentPort @2 :UInt16;
  agentSubId @3 :UInt32;
  sequenceNumber @4 :UInt32;
  uptime @5 :UInt32;
  
  # Sample type (0=FlowSample, 1=CounterSample, 2=ExpandedFlowSample, 3=ExpandedCounterSample)
  sampleType @6 :UInt16;
  
  # Flattened sample data fields (populated based on sampleType)
  sourceId @7 :UInt32;
  samplingRate @8 :UInt32;
  samplePool @9 :UInt32;
  drops @10 :UInt32;
  inputInterface @11 :UInt32;
  outputInterface @12 :UInt32;
  
  # Raw sample data bytes (complete flow/counter sample for future parsing)
  sample @13 :Data;
}

struct FlowSample {
  sourceId @0 :UInt32;
  samplingRate @1 :UInt32;
  samplePool @2 :UInt32;
  drops @3 :UInt32;
  inputInterface @4 :UInt32;
  outputInterface @5 :UInt32;
  flowRecords @6 :List(FlowRecord);
}

struct CounterSample {
  sourceId @0 :UInt32;
  counterRecords @1 :List(CounterRecord);
}

struct FlowRecord {
  union {
    rawPacketHeader @0 :RawPacketHeader;
    ethernetFrame @1 :EthernetFrame;
    ipv4Data @2 :Ipv4Data;
    ipv6Data @3 :Ipv6Data;
    extendedSwitch @4 :ExtendedSwitch;
    extendedRouter @5 :ExtendedRouter;
    extendedGateway @6 :ExtendedGateway;
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

struct Ipv4Data {
  length @0 :UInt32;
  protocol @1 :UInt32;
  srcIp @2 :Data;  # 4 bytes
  dstIp @3 :Data;  # 4 bytes
  srcPort @4 :UInt32;
  dstPort @5 :UInt32;
  tcpFlags @6 :UInt32;
  tos @7 :UInt32;
}

struct Ipv6Data {
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
  asPath @1 :List(UInt32);
  communities @2 :List(UInt32);
  localPref @3 :UInt32;
}

struct CounterRecord {
  union {
    genericInterface @0 :GenericInterface;
    ethernetInterface @1 :EthernetInterface;
  }
}

struct GenericInterface {
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

struct EthernetInterface {
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

use capnp::message::Builder;
use capnp::serialize;
use etherparse::SlicedPacket;
use metrics::counter;
use sflow_parser::models::{Address, FlowData, FlowRecord};
use sflow_parser::{SFlowDatagram, SampleData};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use tracing::trace;

use crate::sflow_capnp::s_flow_flow_record;

pub fn serialize_address(addr: &Address) -> Vec<u8> {
    match addr {
        Address::IPv4(ipv4) => {
            // Convert IPv4 to IPv6-mapped format (16 bytes)
            ipv4.to_ipv6_mapped().octets().to_vec()
        }
        Address::IPv6(ipv6) => ipv6.octets().to_vec(),
        Address::Unknown => {
            // Use unspecified IPv6 address for unknown
            Ipv6Addr::UNSPECIFIED.octets().to_vec()
        }
    }
}

// Parse raw packet header to extract IP information using etherparse
// Returns (length, protocol, src_ip, dst_ip, src_port, dst_port, tcp_flags, tos/priority)
fn parse_raw_packet_header(
    header: &[u8],
) -> Option<(u32, u32, Ipv6Addr, Ipv6Addr, u32, u32, u32, u32)> {
    // Use etherparse to safely parse the packet
    let packet = SlicedPacket::from_ethernet(header).ok()?;

    // Extract IP information
    let (length, protocol, src_ip, dst_ip, tos) = match packet.net {
        Some(etherparse::NetSlice::Ipv4(ipv4)) => {
            let hdr = ipv4.header();
            (
                hdr.total_len() as u32,
                hdr.protocol().0 as u32,
                Ipv4Addr::from(hdr.source()).to_ipv6_mapped(),
                Ipv4Addr::from(hdr.destination()).to_ipv6_mapped(),
                hdr.dcp().value() as u32, // Differentiated Services Code Point
            )
        }
        Some(etherparse::NetSlice::Ipv6(ipv6)) => {
            let hdr = ipv6.header();
            // Use the payload length from the header since we can't easily get actual payload size
            (
                40 + hdr.payload_length() as u32, // IPv6 header is always 40 bytes
                hdr.next_header().0 as u32,
                Ipv6Addr::from(hdr.source()),
                Ipv6Addr::from(hdr.destination()),
                hdr.traffic_class() as u32,
            )
        }
        None => return None,
    };

    // Extract transport layer information (ports and TCP flags)
    let (src_port, dst_port, tcp_flags) = match packet.transport {
        Some(etherparse::TransportSlice::Tcp(tcp)) => {
            let hdr = tcp.to_header();
            (
                hdr.source_port as u32,
                hdr.destination_port as u32,
                hdr.ns as u32
                    | ((hdr.fin as u32) << 1)
                    | ((hdr.syn as u32) << 2)
                    | ((hdr.rst as u32) << 3)
                    | ((hdr.psh as u32) << 4)
                    | ((hdr.ack as u32) << 5)
                    | ((hdr.urg as u32) << 6)
                    | ((hdr.ece as u32) << 7)
                    | ((hdr.cwr as u32) << 8),
            )
        }
        Some(etherparse::TransportSlice::Udp(udp)) => {
            let hdr = udp.to_header();
            (
                hdr.source_port as u32,
                hdr.destination_port as u32,
                0, // No flags for UDP
            )
        }
        _ => (0, 0, 0), // No transport layer or unsupported protocol
    };

    Some((
        length, protocol, src_ip, dst_ip, src_port, dst_port, tcp_flags, tos,
    ))
}

pub fn serialize_sflow_record(
    datagram: &SFlowDatagram,
    time_received_ns: i64,
    peer_addr: SocketAddr,
) -> Vec<Vec<u8>> {
    let mut messages = Vec::new();

    for sample in &datagram.samples {
        match &sample.sample_data {
            SampleData::FlowSample(flow) => {
                process_flow_sample(
                    &mut messages,
                    datagram,
                    time_received_ns,
                    peer_addr,
                    flow.sequence_number,
                    flow.source_id.0,
                    flow.sampling_rate,
                    flow.sample_pool,
                    flow.drops,
                    flow.input.0,
                    flow.output.0,
                    &flow.flow_records,
                );
            }
            SampleData::FlowSampleExpanded(flow) => {
                process_flow_sample(
                    &mut messages,
                    datagram,
                    time_received_ns,
                    peer_addr,
                    flow.sequence_number,
                    flow.source_id.source_id_index,
                    flow.sampling_rate,
                    flow.sample_pool,
                    flow.drops,
                    flow.input.value,
                    flow.output.value,
                    &flow.flow_records,
                );
            }
            SampleData::CountersSample(_) | SampleData::CountersSampleExpanded(_) => {
                // Count counter samples but don't serialize (not supported by schema)
                counter!("pesto_sflow_samples_received_total", "type" => "counter").increment(1);
                trace!("Received counter sample (not serialized)");
            }
            SampleData::DiscardedPacket(_) => {
                trace!("Skipping discarded packet sample");
            }
            SampleData::Unknown { format, .. } => {
                trace!("Skipping unknown sample type: {:?}", format);
            }
        }
    }

    messages
}

#[allow(clippy::too_many_arguments)]
fn process_flow_sample(
    messages: &mut Vec<Vec<u8>>,
    datagram: &SFlowDatagram,
    time_received_ns: i64,
    peer_addr: SocketAddr,
    sample_seq: u32,
    source_id: u32,
    sampling_rate: u32,
    sample_pool: u32,
    drops: u32,
    input_if: u32,
    output_if: u32,
    flow_records: &[FlowRecord],
) {
    // Count flow sample received
    counter!("pesto_sflow_samples_received_total", "type" => "flow").increment(1);

    for flow_record in flow_records {
        // Extract IP data from different flow record types
        // We process all records that contain IP information
        let ip_data: Option<(u32, u32, Ipv6Addr, Ipv6Addr, u32, u32, u32, u32)> =
            match &flow_record.flow_data {
                // Raw packet header (format 1) - most common, contains full packet
                FlowData::SampledHeader(header) => parse_raw_packet_header(&header.header),
                // Direct IP samples (formats 3, 4)
                FlowData::SampledIpv4(ipv4) => Some((
                    ipv4.length,
                    ipv4.protocol,
                    ipv4.src_ip.to_ipv6_mapped(),
                    ipv4.dst_ip.to_ipv6_mapped(),
                    ipv4.src_port,
                    ipv4.dst_port,
                    ipv4.tcp_flags,
                    ipv4.tos,
                )),
                FlowData::SampledIpv6(ipv6) => Some((
                    ipv6.length,
                    ipv6.protocol,
                    ipv6.src_ip,
                    ipv6.dst_ip,
                    ipv6.src_port,
                    ipv6.dst_port,
                    ipv6.tcp_flags,
                    ipv6.priority,
                )),
                // All other flow record types (extended metadata, Ethernet frame info, etc.)
                _ => {
                    // Extended records are metadata only - skip for now
                    // In the future, we could store these separately or merge with flow data
                    trace!(
                        "Skipping extended/metadata flow record: format={:?}",
                        flow_record.flow_format
                    );
                    None
                }
            };

        let (length, protocol, src_ip, dst_ip, src_port, dst_port, tcp_flags, tos) = match ip_data {
            Some(data) => data,
            None => continue,
        };

        let mut message = Builder::new_default();
        {
            let mut record = message.init_root::<s_flow_flow_record::Builder>();

            // Set datagram metadata
            record.set_time_received_ns(time_received_ns as u64);
            record.set_agent_addr(&serialize_address(&datagram.agent_address));
            record.set_agent_port(peer_addr.port());
            record.set_agent_sub_id(datagram.sub_agent_id);
            record.set_datagram_sequence_number(datagram.sequence_number);
            record.set_uptime(datagram.uptime);

            // Set sample metadata
            record.set_sample_sequence_number(sample_seq);
            record.set_source_id(source_id);

            // Set flow sample fields (flat structure)
            record.set_sampling_rate(sampling_rate);
            record.set_sample_pool(sample_pool);
            record.set_drops(drops);
            record.set_input_interface(input_if);
            record.set_output_interface(output_if);

            // Set flow data (all normalized to IPv6)
            record.set_length(length);
            record.set_protocol(protocol);
            record.set_src_ip(&src_ip.octets());
            record.set_dst_ip(&dst_ip.octets());
            record.set_src_port(src_port);
            record.set_dst_port(dst_port);
            record.set_tcp_flags(tcp_flags);
            record.set_tos(tos);
        }

        messages.push(serialize::write_message_to_words(&message));
    }
}

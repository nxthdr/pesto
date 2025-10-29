use capnp::message::Builder;
use capnp::serialize;
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

// Parse raw packet header to extract IP information
// Returns (length, protocol, src_ip, dst_ip, src_port, dst_port, tcp_flags, tos/priority)
fn parse_raw_packet_header(
    header: &[u8],
) -> Option<(u32, u32, Ipv6Addr, Ipv6Addr, u32, u32, u32, u32)> {
    if header.len() < 14 {
        return None; // Too short for Ethernet header
    }

    // Skip Ethernet header (14 bytes: 6 dst MAC + 6 src MAC + 2 EtherType)
    let ether_type = u16::from_be_bytes([header[12], header[13]]);
    let ip_start = 14;

    match ether_type {
        0x0800 => {
            // IPv4
            if header.len() < ip_start + 20 {
                return None; // Too short for IPv4 header
            }

            let ip_header = &header[ip_start..];
            let version_ihl = ip_header[0];
            let ihl = (version_ihl & 0x0F) as usize * 4; // Header length in bytes

            if ihl < 20 || header.len() < ip_start + ihl {
                return None;
            }

            let total_length = u16::from_be_bytes([ip_header[2], ip_header[3]]) as u32;
            let protocol = ip_header[9] as u32;
            let tos = ip_header[1] as u32;

            let src_ip = Ipv4Addr::new(ip_header[12], ip_header[13], ip_header[14], ip_header[15]);
            let dst_ip = Ipv4Addr::new(ip_header[16], ip_header[17], ip_header[18], ip_header[19]);

            // Parse TCP/UDP ports if available
            let (src_port, dst_port, tcp_flags) = if header.len() >= ip_start + ihl + 4 {
                let transport_header = &header[ip_start + ihl..];
                let src = u16::from_be_bytes([transport_header[0], transport_header[1]]) as u32;
                let dst = u16::from_be_bytes([transport_header[2], transport_header[3]]) as u32;

                // Get TCP flags if it's TCP
                let flags = if protocol == 6 && header.len() >= ip_start + ihl + 14 {
                    transport_header[13] as u32
                } else {
                    0
                };

                (src, dst, flags)
            } else {
                (0, 0, 0)
            };

            Some((
                total_length,
                protocol,
                src_ip.to_ipv6_mapped(),
                dst_ip.to_ipv6_mapped(),
                src_port,
                dst_port,
                tcp_flags,
                tos,
            ))
        }
        0x86DD => {
            // IPv6
            if header.len() < ip_start + 40 {
                return None; // Too short for IPv6 header
            }

            let ip_header = &header[ip_start..];
            let payload_length = u16::from_be_bytes([ip_header[4], ip_header[5]]) as u32;
            let next_header = ip_header[6] as u32; // Protocol
            let traffic_class = ((ip_header[0] & 0x0F) << 4 | (ip_header[1] >> 4)) as u32;

            let src_ip = Ipv6Addr::from([
                ip_header[8],
                ip_header[9],
                ip_header[10],
                ip_header[11],
                ip_header[12],
                ip_header[13],
                ip_header[14],
                ip_header[15],
                ip_header[16],
                ip_header[17],
                ip_header[18],
                ip_header[19],
                ip_header[20],
                ip_header[21],
                ip_header[22],
                ip_header[23],
            ]);
            let dst_ip = Ipv6Addr::from([
                ip_header[24],
                ip_header[25],
                ip_header[26],
                ip_header[27],
                ip_header[28],
                ip_header[29],
                ip_header[30],
                ip_header[31],
                ip_header[32],
                ip_header[33],
                ip_header[34],
                ip_header[35],
                ip_header[36],
                ip_header[37],
                ip_header[38],
                ip_header[39],
            ]);

            // Parse TCP/UDP ports if available
            let (src_port, dst_port, tcp_flags) = if header.len() >= ip_start + 40 + 4 {
                let transport_header = &header[ip_start + 40..];
                let src = u16::from_be_bytes([transport_header[0], transport_header[1]]) as u32;
                let dst = u16::from_be_bytes([transport_header[2], transport_header[3]]) as u32;

                // Get TCP flags if it's TCP
                let flags = if next_header == 6 && header.len() >= ip_start + 40 + 14 {
                    transport_header[13] as u32
                } else {
                    0
                };

                (src, dst, flags)
            } else {
                (0, 0, 0)
            };

            Some((
                40 + payload_length, // IPv6 header + payload
                next_header,
                src_ip,
                dst_ip,
                src_port,
                dst_port,
                tcp_flags,
                traffic_class,
            ))
        }
        _ => None, // Unsupported EtherType
    }
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

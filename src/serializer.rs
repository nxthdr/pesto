use capnp::message::Builder;
use capnp::serialize;
use sflow_parser::models::{Address, FlowData};
use sflow_parser::{SFlowDatagram, SampleData};
use std::net::{Ipv6Addr, SocketAddr};

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

pub fn serialize_sflow_record(
    datagram: &SFlowDatagram,
    time_received_ns: i64,
    peer_addr: SocketAddr,
) -> Vec<Vec<u8>> {
    let mut messages = Vec::new();

    for sample in &datagram.samples {
        // Only process flow samples
        let (sample_seq, source_id, sampling_rate, sample_pool, drops, input_if, output_if, flow_records) = match &sample.sample_data {
            SampleData::FlowSample(flow) => (
                flow.sequence_number,
                flow.source_id.0,
                flow.sampling_rate,
                flow.sample_pool,
                flow.drops,
                flow.input.0,
                flow.output.0,
                &flow.flow_records,
            ),
            SampleData::FlowSampleExpanded(flow) => (
                flow.sequence_number,
                flow.source_id.source_id_index,
                flow.sampling_rate,
                flow.sample_pool,
                flow.drops,
                flow.input.value,
                flow.output.value,
                &flow.flow_records,
            ),
            _ => continue, // Skip counter samples for now
        };

        // Create one message per flow record
        for flow_record in flow_records {
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
                record.set_sampling_rate(sampling_rate);
                record.set_sample_pool(sample_pool);
                record.set_drops(drops);
                record.set_input_interface(input_if);
                record.set_output_interface(output_if);
                
                // Set flow record data based on type
                // Always use IPv6 format (map IPv4 to IPv6)
                match &flow_record.flow_data {
                    FlowData::SampledIpv4(ipv4) => {
                        // Convert IPv4 to IPv6-mapped and store in sampledIpv6
                        let mut ipv6_builder = record.init_sampled_ipv6();
                        ipv6_builder.set_length(ipv4.length);
                        ipv6_builder.set_protocol(ipv4.protocol);
                        ipv6_builder.set_src_ip(&ipv4.src_ip.to_ipv6_mapped().octets());
                        ipv6_builder.set_dst_ip(&ipv4.dst_ip.to_ipv6_mapped().octets());
                        ipv6_builder.set_src_port(ipv4.src_port);
                        ipv6_builder.set_dst_port(ipv4.dst_port);
                        ipv6_builder.set_tcp_flags(ipv4.tcp_flags);
                        ipv6_builder.set_priority(ipv4.tos); // Map TOS to priority
                    }
                    FlowData::SampledIpv6(ipv6) => {
                        let mut ipv6_builder = record.init_sampled_ipv6();
                        ipv6_builder.set_length(ipv6.length);
                        ipv6_builder.set_protocol(ipv6.protocol);
                        ipv6_builder.set_src_ip(&ipv6.src_ip.octets());
                        ipv6_builder.set_dst_ip(&ipv6.dst_ip.octets());
                        ipv6_builder.set_src_port(ipv6.src_port);
                        ipv6_builder.set_dst_port(ipv6.dst_port);
                        ipv6_builder.set_tcp_flags(ipv6.tcp_flags);
                        ipv6_builder.set_priority(ipv6.priority);
                    }
                    _ => {
                        // Skip other flow record types for now
                        continue;
                    }
                }
            }

            messages.push(serialize::write_message_to_words(&message));
        }
    }

    messages
}

use capnp::message::Builder;
use capnp::serialize;
use sflow_parser::models::Address;
use sflow_parser::{SFlowDatagram, SampleData};
use std::net::{Ipv6Addr, SocketAddr};

use crate::sflow_capnp::s_flow_record;

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
        let mut message = Builder::new_default();
        {
            let mut record = message.init_root::<s_flow_record::Builder>();
            
            // Set metadata
            record.set_time_received_ns(time_received_ns as u64);
            record.set_agent_addr(&serialize_address(&datagram.agent_address));
            record.set_agent_port(peer_addr.port());
            record.set_agent_sub_id(datagram.sub_agent_id);
            record.set_sequence_number(datagram.sequence_number);
            record.set_uptime(datagram.uptime);

            // Set sample type and flattened data
            match &sample.sample_data {
                SampleData::FlowSample(flow) => {
                    record.set_sample_type(0); // FlowSample
                    record.set_source_id(flow.source_id.0);
                    record.set_sampling_rate(flow.sampling_rate);
                    record.set_sample_pool(flow.sample_pool);
                    record.set_drops(flow.drops);
                    record.set_input_interface(flow.input.0);
                    record.set_output_interface(flow.output.0);
                    
                    // Store sample data as binary (bincode serialization)
                    if let Ok(bytes) = bincode::serialize(flow) {
                        record.set_sample(&bytes);
                    }
                }
                SampleData::CountersSample(counter) => {
                    record.set_sample_type(1); // CounterSample
                    record.set_source_id(counter.source_id.0);
                    record.set_sampling_rate(0);
                    record.set_sample_pool(0);
                    record.set_drops(0);
                    record.set_input_interface(0);
                    record.set_output_interface(0);
                    
                    // Store sample data as debug string (for future parsing)
                    let debug_str = format!("{:?}", counter);
                    record.set_sample(debug_str.as_bytes());
                }
                SampleData::FlowSampleExpanded(flow) => {
                    record.set_sample_type(2); // ExpandedFlowSample
                    // For expanded samples, combine source_id_type and source_id_index
                    record.set_source_id((flow.source_id.source_id_type << 24) | (flow.source_id.source_id_index & 0xFFFFFF));
                    record.set_sampling_rate(flow.sampling_rate);
                    record.set_sample_pool(flow.sample_pool);
                    record.set_drops(flow.drops);
                    record.set_input_interface((flow.input.format << 30) | (flow.input.value & 0x3FFFFFFF));
                    record.set_output_interface((flow.output.format << 30) | (flow.output.value & 0x3FFFFFFF));
                    
                    // Store sample data as binary (bincode serialization)
                    if let Ok(bytes) = bincode::serialize(flow) {
                        record.set_sample(&bytes);
                    }
                }
                SampleData::CountersSampleExpanded(counter) => {
                    record.set_sample_type(3); // ExpandedCounterSample
                    // For expanded samples, combine source_id_type and source_id_index
                    record.set_source_id((counter.source_id.source_id_type << 24) | (counter.source_id.source_id_index & 0xFFFFFF));
                    record.set_sampling_rate(0);
                    record.set_sample_pool(0);
                    record.set_drops(0);
                    record.set_input_interface(0);
                    record.set_output_interface(0);
                    
                    // Store sample data as debug string (for future parsing)
                    let debug_str = format!("{:?}", counter);
                    record.set_sample(debug_str.as_bytes());
                }
                SampleData::DiscardedPacket(_) | SampleData::Unknown { .. } => {
                    // Skip unsupported sample types for now
                    continue;
                }
            }
        }

        messages.push(serialize::write_message_to_words(&message));
    }

    messages
}

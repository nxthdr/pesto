# Pesto

High-performance sFlow v5 collector that receives sFlow datagrams via UDP, parses them, and forwards them to Kafka as Cap'n Proto messages.

## Usage

```bash
# Basic usage with default settings (listens on 0.0.0.0:6343)
pesto

# Custom sFlow listener address
pesto --sflow-address 0.0.0.0:6343

# Configure Kafka brokers
pesto --kafka-brokers broker1:9092,broker2:9092 --kafka-topic pesto-sflow

# Disable Kafka producer (for testing)
pesto --kafka-disable

# Enable verbose logging
pesto -vvv
```

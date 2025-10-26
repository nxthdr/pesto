FROM rust:1.83-bookworm AS builder

WORKDIR /build

# Copy the producer source
COPY . .

# Build the producer
RUN cargo build --release

FROM debian:bookworm-slim

COPY --from=builder /build/target/release/sflow-producer /usr/local/bin/sflow-producer

ENTRYPOINT ["/usr/local/bin/sflow-producer"]

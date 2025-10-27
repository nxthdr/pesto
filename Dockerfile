FROM lukemathwalker/cargo-chef:latest-rust-1-trixie AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt-get update \
    && apt-get install -y capnproto libsasl2-dev libssl-dev librdkafka-dev \
    && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin pesto

FROM debian:trixie-slim AS runtime
RUN apt-get update \
    && apt-get install -y openssl libsasl2-2 libsasl2-modules ca-certificates \
    && update-ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/pesto /app/pesto

EXPOSE 4000
EXPOSE 8080

ENTRYPOINT [ "/app/pesto" ]

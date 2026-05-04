FROM rust:1.88 AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/live_events_service /usr/local/bin/live_events_service
EXPOSE 50055
CMD ["live_events_service"]

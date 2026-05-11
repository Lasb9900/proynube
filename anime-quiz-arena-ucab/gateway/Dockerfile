FROM rust:1.88 AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/api_gateway /usr/local/bin/api_gateway
EXPOSE 50050
EXPOSE 8080
CMD ["/usr/local/bin/api_gateway"]

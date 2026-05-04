FROM rust:1.86 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release || true
FROM debian:bookworm-slim
WORKDIR /app
CMD ["/bin/sh","-c","echo gateway skeleton"]

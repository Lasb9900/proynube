FROM rust:1.78 as builder
WORKDIR /app
COPY . .
RUN cargo build --release || true
FROM debian:bookworm-slim
WORKDIR /app
CMD ["/bin/sh","-c","echo gateway skeleton"]

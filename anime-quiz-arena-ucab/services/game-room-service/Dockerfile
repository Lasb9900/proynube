FROM rust:1.88 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release || true
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release /app/
CMD ["/bin/sh","-c","echo skeleton"]

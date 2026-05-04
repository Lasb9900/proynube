FROM rust:1.78 as builder
WORKDIR /app
COPY . .
RUN cargo build --release || true
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release /app/
CMD ["/bin/sh","-c","echo skeleton"]

# Builder stage
FROM rust:1.77-slim AS builder

WORKDIR /app
# Устанавливаем зависимости для сборки (если потребуются для OpenSSL и прочего)
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app
# Устанавливаем корневые сертификаты для возможных TLS соединений
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/worker /usr/local/bin/worker

ENTRYPOINT ["worker"]
# По умолчанию слушаем на всех интерфейсах (внутри контейнера)
CMD ["--listen-type", "tcp", "--listen-addr", "0.0.0.0:8080"]

FROM rust:1.45 as builder
WORKDIR /usr/src/vaas-server
COPY . .
RUN SQLX_OFFLINE=true cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libssl1.1 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/vaas-server /usr/local/bin/vaas-server
CMD ["vaas-server"]

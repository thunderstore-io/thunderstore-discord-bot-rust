FROM rust:latest as builder
WORKDIR /thunderstore-discord-bot-rust
COPY . .
RUN cargo install --locked --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y openssl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/thunderstore-discord-bot-rust /usr/local/bin/thunderstore-discord-bot-rust
CMD ["thunderstore-discord-bot-rust"]

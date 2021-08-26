FROM rust:latest as builder

WORKDIR /thunderstore-discord-bot-rust

# Cache build dependencies
COPY Cargo.toml .
COPY Cargo.lock .
COPY dummy.rs .
RUN sed -i 's/src\/main.rs/dummy.rs/' Cargo.toml
RUN mkdir .cargo
RUN cargo build --release
RUN sed -i 's/dummy.rs/src\/main.rs/' Cargo.toml

# Actually build our package
COPY . .
RUN cargo build --release
RUN cargo install --locked --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y openssl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/thunderstore-discord-bot-rust /usr/local/bin/thunderstore-discord-bot-rust
CMD ["thunderstore-discord-bot-rust"]

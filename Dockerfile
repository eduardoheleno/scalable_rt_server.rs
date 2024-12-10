# local development
# FROM rust

# RUN cargo install cargo-watch --locked

# WORKDIR /usr/src/server
# COPY . .

# CMD ["cargo", "watch", "-w", "src/", "-x", "run"]

FROM rust:latest AS builder

WORKDIR /usr/src/server

COPY . .

RUN cargo build --release -j 4

FROM debian:bookworm

RUN apt update
RUN apt install libc6

COPY --from=builder /usr/src/server/target/release/server /usr/local/bin/server

EXPOSE 80

ENTRYPOINT ["/usr/local/bin/server"]

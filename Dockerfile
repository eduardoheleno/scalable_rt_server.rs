FROM rust:latest AS builder

WORKDIR /usr/src/server

COPY . .

RUN cargo build --release

FROM debian:bookworm

RUN apt update
RUN apt install libc6

COPY --from=builder /usr/src/server/target/release/server /usr/local/bin/server

EXPOSE 80

ENTRYPOINT ["/usr/local/bin/server"]

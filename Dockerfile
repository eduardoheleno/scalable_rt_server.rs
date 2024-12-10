# local development
FROM rust

RUN cargo install cargo-watch --locked

WORKDIR /usr/src/server
COPY . .

CMD ["cargo", "watch", "-w", "src/", "-x", "run"]

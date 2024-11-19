FROM rust

WORKDIR /usr/src/server
COPY . .

RUN cargo install cargo-watch

CMD ["cargo", "watch", "-w", "src/", "-x", "run"]

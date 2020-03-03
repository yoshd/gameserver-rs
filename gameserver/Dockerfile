FROM gameserver-base as builder

# for cache
COPY Cargo.toml Cargo.lock /home/builder/gameserver-rs/gameserver/
RUN mkdir /home/builder/gameserver-rs/gameserver/src && echo "fn main() {}" >> /home/builder/gameserver-rs/gameserver/src/main.rs
WORKDIR /home/builder/gameserver-rs/gameserver
RUN cargo build --release

COPY . /home/builder/gameserver-rs/gameserver
RUN cargo build --release

FROM debian:stretch
RUN useradd -m server

COPY --from=builder /home/builder/gameserver-rs/gameserver/target/release/gameserver /home/server/gameserver
RUN chown -R server /home/server && \
    chmod o+x /home/server/gameserver

USER server
ENTRYPOINT /home/server/gameserver

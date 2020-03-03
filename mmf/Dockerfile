FROM gameserver-base as builder

# for cache
COPY Cargo.toml Cargo.lock /home/builder/gameserver-rs/mmf/
RUN mkdir /home/builder/gameserver-rs/mmf/src && echo "fn main() {}" >> /home/builder/gameserver-rs/mmf/src/main.rs
WORKDIR /home/builder/gameserver-rs/mmf
RUN cargo build --release

COPY . /home/builder/gameserver-rs/mmf
RUN cargo build --release

FROM debian:stretch
RUN useradd -m server

COPY --from=builder /home/builder/gameserver-rs/mmf/target/release/matchfunction /home/server/matchfunction
RUN chown -R server /home/server && \
    chmod o+x /home/server/matchfunction

USER server
ENTRYPOINT /home/server/matchfunction

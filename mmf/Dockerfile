FROM gameserver-base as builder

COPY . /home/builder/gameserver-rs/mmf
WORKDIR /home/builder/gameserver-rs/mmf
RUN cargo build --release

FROM debian:stretch
RUN useradd -m server

COPY --from=builder /home/builder/gameserver-rs/mmf/target/release/matchfunction /home/server/matchfunction
RUN chown -R server /home/server && \
    chmod o+x /home/server/matchfunction

USER server
ENTRYPOINT /home/server/matchfunction
